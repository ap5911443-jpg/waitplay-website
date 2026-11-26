-- Vibe Play account data: profiles, plan/entitlements, devices, user preferences.
-- Every table has Row Level Security ENABLED and FORCED. `anon` gets nothing. Signed-in users can only ever see their own rows.
-- Run it once (Supabase dashboard > SQL editor, or `supabase db push`). It is idempotent where practical.

-- ── tables ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────

create table if not exists public.profiles (
  id           uuid primary key references auth.users (id) on delete cascade,
  display_name text not null default '' check (char_length(display_name) <= 80),
  avatar_url   text check (avatar_url is null or char_length(avatar_url) <= 500),
  status       text not null default 'active' check (status in ('active', 'suspended')),
  created_at   timestamptz not null default now(),
  updated_at   timestamptz not null default now()
);

create table if not exists public.entitlements (
  user_id              uuid primary key references auth.users (id) on delete cascade,
  plan                 text not null default 'free' check (plan in ('free', 'pro')),
  status               text not null default 'active' check (status in ('active', 'trialing', 'past_due', 'canceled')),
  device_limit         integer not null default 2 check (device_limit between 1 and 100),
  current_period_end   timestamptz,
  provider             text,
  provider_customer_id text,
  updated_at           timestamptz not null default now()
);

create table if not exists public.devices (
  id          uuid primary key default gen_random_uuid(),
  user_id     uuid not null references auth.users (id) on delete cascade,
  device_id   text not null check (char_length(device_id) between 16 and 64),
  name        text not null default '' check (char_length(name) <= 80),
  platform    text not null check (platform in ('windows', 'macos', 'other')),
  app_version text not null default '' check (char_length(app_version) <= 32),
  first_seen  timestamptz not null default now(),
  last_seen   timestamptz not null default now(),
  revoked_at  timestamptz,
  unique (user_id, device_id)
);
create index if not exists devices_user_idx on public.devices (user_id);

create table if not exists public.user_preferences (
  user_id    uuid primary key references auth.users (id) on delete cascade,
  data       jsonb not null default '{}'::jsonb check (pg_column_size(data) <= 65536),
  version    integer not null default 1,
  updated_at timestamptz not null default now()
);

-- ── row level security ───────────────────────────────────────────────────────────────────────────────────────────────────────

alter table public.profiles         enable row level security;
alter table public.entitlements     enable row level security;
alter table public.devices          enable row level security;
alter table public.user_preferences enable row level security;
alter table public.profiles         force row level security;
alter table public.entitlements     force row level security;
alter table public.devices          force row level security;
alter table public.user_preferences force row level security;

-- start from nothing, then grant exactly what the app needs
revoke all on public.profiles, public.entitlements, public.devices, public.user_preferences from anon, authenticated;

grant select on public.profiles to authenticated;
grant update (display_name, avatar_url) on public.profiles to authenticated;      -- not `status`
grant select on public.entitlements to authenticated;                              -- written only by the service role (billing)
grant select on public.devices to authenticated;                                   -- written only through register_device()/revoke_device()
grant select, insert, update, delete on public.user_preferences to authenticated;

drop policy if exists profiles_select_own on public.profiles;
create policy profiles_select_own on public.profiles for select to authenticated using (id = (select auth.uid()));
drop policy if exists profiles_update_own on public.profiles;
create policy profiles_update_own on public.profiles for update to authenticated
  using (id = (select auth.uid())) with check (id = (select auth.uid()));

drop policy if exists entitlements_select_own on public.entitlements;
create policy entitlements_select_own on public.entitlements for select to authenticated using (user_id = (select auth.uid()));

drop policy if exists devices_select_own on public.devices;
create policy devices_select_own on public.devices for select to authenticated using (user_id = (select auth.uid()));

drop policy if exists preferences_own on public.user_preferences;
create policy preferences_own on public.user_preferences for all to authenticated
  using (user_id = (select auth.uid())) with check (user_id = (select auth.uid()));

-- ── triggers ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────

create or replace function public.touch_updated_at() returns trigger
language plpgsql set search_path = '' as $$
begin
  new.updated_at := now();
  return new;
end;
$$;

drop trigger if exists profiles_touch on public.profiles;
create trigger profiles_touch before update on public.profiles for each row execute function public.touch_updated_at();
drop trigger if exists preferences_touch on public.user_preferences;
create trigger preferences_touch before update on public.user_preferences for each row execute function public.touch_updated_at();

-- every new auth user gets a profile and a free entitlement
create or replace function public.handle_new_user() returns trigger
language plpgsql security definer set search_path = '' as $$
begin
  insert into public.profiles (id, display_name)
  values (new.id, left(coalesce(new.raw_user_meta_data ->> 'full_name', new.raw_user_meta_data ->> 'name', ''), 80))
  on conflict (id) do nothing;
  insert into public.entitlements (user_id) values (new.id) on conflict (user_id) do nothing;
  return new;
end;
$$;
revoke all on function public.handle_new_user() from public, anon, authenticated;

drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created after insert on auth.users for each row execute function public.handle_new_user();

-- users that existed before this migration
insert into public.profiles (id) select id from auth.users on conflict (id) do nothing;
insert into public.entitlements (user_id) select id from auth.users on conflict (user_id) do nothing;

-- ── devices: the only write path ─────────────────────────────────────────────────────────────────────────────────────────────

-- Returns 'ok', 'limit_reached' (the plan's device limit is used up) or 'revoked' (this device was removed from the account).
-- A background heartbeat (p_explicit = false) never revives a revoked device; an explicit sign-in on that device (p_explicit = true) may,
-- if the plan still has a free slot.
drop function if exists public.register_device(text, text, text, text);
create or replace function public.register_device(p_device_id text, p_name text, p_platform text, p_app_version text, p_explicit boolean default false)
returns text language plpgsql security definer set search_path = '' as $$
declare
  uid uuid := (select auth.uid());
  lim integer;
  existing public.devices%rowtype;
  known boolean;
  active_count integer;
begin
  if uid is null then
    raise exception 'not authenticated' using errcode = '28000';
  end if;
  if p_device_id is null or char_length(p_device_id) not between 16 and 64 or p_platform not in ('windows', 'macos', 'other') then
    raise exception 'invalid device' using errcode = '22023';
  end if;

  perform pg_advisory_xact_lock(hashtextextended(uid::text, 0));   -- two devices registering at once cannot both pass the limit check

  select * into existing from public.devices where user_id = uid and device_id = p_device_id;
  known := found;   -- FOUND is overwritten by every later SELECT INTO
  if known and existing.revoked_at is null then
    update public.devices
       set last_seen = now(), name = left(coalesce(p_name, ''), 80), app_version = left(coalesce(p_app_version, ''), 32)
     where id = existing.id;
    return 'ok';
  end if;
  if known and not coalesce(p_explicit, false) then
    return 'revoked';
  end if;

  select coalesce((select device_limit from public.entitlements where user_id = uid), 2) into lim;
  select count(*) into active_count from public.devices where user_id = uid and revoked_at is null;
  if active_count >= lim then
    return case when known then 'revoked' else 'limit_reached' end;
  end if;

  if known then
    update public.devices set revoked_at = null, last_seen = now(), name = left(coalesce(p_name, ''), 80), app_version = left(coalesce(p_app_version, ''), 32) where id = existing.id;
    return 'ok';
  end if;

  insert into public.devices (user_id, device_id, name, platform, app_version)
  values (uid, p_device_id, left(coalesce(p_name, ''), 80), p_platform, left(coalesce(p_app_version, ''), 32));
  return 'ok';
end;
$$;

create or replace function public.revoke_device(p_id uuid) returns boolean
language plpgsql security definer set search_path = '' as $$
declare
  uid uuid := (select auth.uid());
  n integer;
begin
  if uid is null then
    raise exception 'not authenticated' using errcode = '28000';
  end if;
  update public.devices set revoked_at = now() where id = p_id and user_id = uid and revoked_at is null;
  get diagnostics n = row_count;
  return n > 0;
end;
$$;

revoke all on function public.register_device(text, text, text, text, boolean) from public, anon;
revoke all on function public.revoke_device(uuid) from public, anon;
grant execute on function public.register_device(text, text, text, text, boolean) to authenticated;
grant execute on function public.revoke_device(uuid) to authenticated;
