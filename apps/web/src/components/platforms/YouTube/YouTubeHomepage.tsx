import React, { useState, useMemo } from 'react';
import {
  Menu,
  Search,
  Mic,
  Video,
  Bell,
  Home,
  Flame,
  Tv,
  Compass,
  History,
  Clock,
  ThumbsUp,
  FileVideo,
  User,
  MoreVertical,
  CheckCircle2,
  X,
} from 'lucide-react';
import { YouTubeVideoPlayer, YouTubeVideoData } from './YouTubeVideoPlayer';

const CATEGORIES = [
  'All',
  'AI',
  'Startup company',
  'Podcasts',
  'Web series',
  'Sitcoms',
  'Music',
  'Mixes',
  'Satire',
  'Romantic comedies',
  'Thrillers',
  'Masala films',
  'Theatre',
  'Live',
  'Stocks',
  'Information',
];

const INITIAL_VIDEOS: YouTubeVideoData[] = [
  {
    id: 'yt-1',
    title: 'AI Marketing: Big Money — How Companies Automate Millions',
    category: 'AI',
    duration: '18:24',
    views: '1.4M views',
    uploadTime: '2 days ago',
    channel: 'Tech Unplugged',
    subscribers: '2.1M subscribers',
    channelAvatarText: 'TU',
    channelAvatarBg: '#2563EB',
    isVerified: true,
    thumbnailGradient: 'linear-gradient(135deg, #1E1B4B 0%, #312E81 50%, #1E3A8A 100%)',
    thumbnailTag: 'AI MARKETING',
    description: 'An investigative breakdown into how modern enterprise SaaS companies leverage agentic systems to scale revenue with zero human intervention.',
  },
  {
    id: 'yt-2',
    title: 'Dhindora | EP 02 — The Neighborhood Mystery',
    category: 'Web series',
    duration: '24:15',
    views: '28M views',
    uploadTime: '3 years ago',
    channel: 'BB Ki Vines',
    subscribers: '26M subscribers',
    channelAvatarText: 'BB',
    channelAvatarBg: '#DC2626',
    isVerified: true,
    thumbnailGradient: 'linear-gradient(135deg, #450A0A 0%, #7F1D1D 50%, #991B1B 100%)',
    thumbnailTag: 'DHINDORA EP 02',
    description: 'Episode 2 of the blockbuster web series Dhindora. Watch the chaos unfold as lottery fever takes over the colony.',
  },
  {
    id: 'yt-3',
    title: 'How Open Source Founders Make Millions in Modern Software',
    category: 'Startup company',
    duration: '14:02',
    views: '890K views',
    uploadTime: '1 month ago',
    channel: 'Y Combinator',
    subscribers: '1.4M subscribers',
    channelAvatarText: 'YC',
    channelAvatarBg: '#EA580C',
    isVerified: true,
    thumbnailGradient: 'linear-gradient(135deg, #431407 0%, #9A3412 50%, #C2410C 100%)',
    thumbnailTag: 'YC FOUNDER INTERVIEW',
    description: 'YC partners interview open-source founders on enterprise open core models, dual licensing, and scaling developer communities into unicorns.',
  },
  {
    id: 'yt-4',
    title: '9 Minutes of Non-stop Laughter — Standup Comedy Special',
    category: 'Satire',
    duration: '09:12',
    views: '4.2M views',
    uploadTime: '5 months ago',
    channel: 'Stand-up Central',
    subscribers: '3.8M subscribers',
    channelAvatarText: 'SC',
    channelAvatarBg: '#CA8A04',
    isVerified: true,
    thumbnailGradient: 'linear-gradient(135deg, #422006 0%, #854D0E 50%, #A16207 100%)',
    thumbnailTag: 'BEST STANDUP JOKES',
    description: 'The funniest 9 minutes of unfiltered observational comedy and relatable workplace moments.',
  },
  {
    id: 'yt-5',
    title: "Mastercard's Invisible Empire: How Payment Rails Rule Finance",
    category: 'Information',
    duration: '21:40',
    views: '3.1M views',
    uploadTime: '2 weeks ago',
    channel: 'ColdFusion',
    subscribers: '4.9M subscribers',
    channelAvatarText: 'CF',
    channelAvatarBg: '#059669',
    isVerified: true,
    thumbnailGradient: 'linear-gradient(135deg, #064E3B 0%, #065F46 50%, #047857 100%)',
    thumbnailTag: 'COLDFUSION INVESTIGATES',
    description: 'How a 0.5% interchange fee created one of the most profitable and untouchable financial monopolies on planet earth.',
  },
  {
    id: 'yt-6',
    title: 'Autonomous AI Coding Agents: The Next Paradigm Shift',
    category: 'AI',
    duration: '45:10',
    views: '1.8M views',
    uploadTime: '1 week ago',
    channel: 'Andrej Karpathy',
    subscribers: '1.2M subscribers',
    channelAvatarText: 'AK',
    channelAvatarBg: '#7C3AED',
    isVerified: true,
    thumbnailGradient: 'linear-gradient(135deg, #2E1065 0%, #581C87 50%, #6B21A8 100%)',
    thumbnailTag: 'DEEP LEARNING & AGENTS',
    description: 'Walking through multi-step agent reasoning, terminal tool use, state machines, and why micro-break companions keep developer momentum alive.',
  },
  {
    id: 'yt-7',
    title: 'Building a $100k/mo Micro-SaaS Solo: Complete Roadmap',
    category: 'Startup company',
    duration: '16:30',
    views: '650K views',
    uploadTime: '4 days ago',
    channel: 'Starter Story',
    subscribers: '890K subscribers',
    channelAvatarText: 'SS',
    channelAvatarBg: '#0891B2',
    isVerified: true,
    thumbnailGradient: 'linear-gradient(135deg, #164E63 0%, #0E7490 50%, #0284C7 100%)',
    thumbnailTag: 'MICRO-SAAS BREAKDOWN',
    description: 'Step by step breakdown of validating, coding with AI tools, and scaling a single-person SaaS to six figures monthly recurring revenue.',
  },
  {
    id: 'yt-8',
    title: 'The Future of Frontend & LLMs: Streaming Interfaces',
    category: 'Web series',
    duration: '12:48',
    views: '410K views',
    uploadTime: '3 weeks ago',
    channel: 'Guillermo Rauch',
    subscribers: '320K subscribers',
    channelAvatarText: 'GR',
    channelAvatarBg: '#000000',
    isVerified: true,
    thumbnailGradient: 'linear-gradient(135deg, #18181B 0%, #27272A 50%, #3F3F46 100%)',
    thumbnailTag: 'FRONTEND ARCHITECTURE',
    description: 'How generative UI and streaming tokens change web development forever. Real-time rendering with edge networks.',
  },
  {
    id: 'yt-9',
    title: 'Lo-Fi Hip Hop Beats to Code & Relax to [24/7 Live Stream]',
    category: 'Live',
    duration: 'LIVE',
    views: '84K watching',
    uploadTime: 'Started streaming 3 days ago',
    channel: 'Lofi Girl',
    subscribers: '14.5M subscribers',
    channelAvatarText: 'LG',
    channelAvatarBg: '#DB2777',
    isVerified: true,
    thumbnailGradient: 'linear-gradient(135deg, #831843 0%, #9D174D 50%, #BE185D 100%)',
    thumbnailTag: 'LO-FI HIP HOP STREAM',
    description: 'Peaceful lofi beats to study, code, relax, and chill to. Ambient background music designed for deep work focus.',
  },
];

export const YouTubeHomepage: React.FC = () => {
  const [videos] = useState<YouTubeVideoData[]>(INITIAL_VIDEOS);
  const [selectedCategory, setSelectedCategory] = useState('All');
  const [searchQuery, setSearchQuery] = useState('');
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);
  const [activeVideo, setActiveVideo] = useState<YouTubeVideoData | null>(null);

  const filteredVideos = useMemo(() => {
    return videos.filter((v) => {
      const matchesCategory =
        selectedCategory === 'All' || v.category.toLowerCase() === selectedCategory.toLowerCase();
      const matchesSearch =
        !searchQuery.trim() ||
        v.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        v.channel.toLowerCase().includes(searchQuery.toLowerCase()) ||
        v.thumbnailTag.toLowerCase().includes(searchQuery.toLowerCase());
      return matchesCategory && matchesSearch;
    });
  }, [videos, selectedCategory, searchQuery]);

  if (activeVideo) {
    return (
      <YouTubeVideoPlayer
        video={activeVideo}
        onBack={() => setActiveVideo(null)}
        onSelectRelated={(v) => setActiveVideo(v)}
        relatedVideos={videos.filter((v) => v.id !== activeVideo.id)}
      />
    );
  }

  return (
    <div className="w-full h-full bg-[#0F0F0F] text-[#F1F1F1] flex flex-col overflow-hidden font-sans select-none">
      {/* A. Fixed Top Navigation Bar (spec) */}
      <header className="sticky top-0 z-30 h-14 bg-[#0F0F0F] px-4 flex items-center justify-between border-b border-[#303030]/60 shrink-0">
        {/* Left: Hamburger + YouTube Logo & Wordmark */}
        <div className="flex items-center gap-4">
          <button
            onClick={() => setIsSidebarOpen(!isSidebarOpen)}
            className="p-2 rounded-full hover:bg-[#272727] text-white transition-colors cursor-pointer"
            title="Toggle Sidebar"
          >
            <Menu size={20} />
          </button>

          <div
            onClick={() => {
              setSelectedCategory('All');
              setSearchQuery('');
            }}
            className="flex items-center gap-1.5 cursor-pointer"
          >
            {/* Red Play Wordmark Badge */}
            <div className="w-7 h-5 rounded-md bg-[#FF0000] flex items-center justify-center shadow-xs">
              <div className="w-0 h-0 border-y-[4px] border-y-transparent border-l-[7px] border-l-white ml-0.5" />
            </div>
            <span className="font-bold tracking-tighter text-lg text-white">YouTube</span>
            <span className="text-[10px] text-[#AAAAAA] -mt-2.5 font-medium ml-0.5">Vibe Play</span>
          </div>
        </div>

        {/* Center: Wide Search Section with Mic */}
        <div className="flex-1 max-w-xl mx-4 hidden sm:flex items-center justify-center">
          <div className="flex items-center w-full max-w-md bg-[#121212] border border-[#303030] focus-within:border-[#1C62B9] rounded-l-full overflow-hidden px-4 py-1.5">
            <Search size={16} className="text-[#AAAAAA] mr-2.5 shrink-0" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search"
              className="w-full bg-transparent text-sm text-[#F1F1F1] placeholder-[#AAAAAA] outline-none"
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery('')}
                className="text-[#AAAAAA] hover:text-white p-0.5 mr-1"
              >
                <X size={15} />
              </button>
            )}
          </div>
          <button
            className="px-5 py-2 rounded-r-full bg-[#222222] hover:bg-[#272727] border border-l-0 border-[#303030] text-[#AAAAAA] hover:text-white transition-colors cursor-pointer"
            title="Search"
          >
            <Search size={16} />
          </button>
          <button
            className="ml-3 p-2.5 rounded-full bg-[#222222] hover:bg-[#272727] text-white transition-colors cursor-pointer"
            title="Search with your voice"
          >
            <Mic size={16} />
          </button>
        </div>

        {/* Right Section: Create, Notifications, Profile Avatar */}
        <div className="flex items-center gap-2">
          <button
            className="hidden md:flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-[#272727] hover:bg-[#3F3F3F] text-xs font-semibold text-white transition-colors cursor-pointer"
          >
            <Video size={16} />
            <span>Create</span>
          </button>

          <button
            className="relative p-2 rounded-full hover:bg-[#272727] text-white transition-colors cursor-pointer"
            title="Notifications"
          >
            <Bell size={20} />
            <span className="absolute top-1 right-1 w-4 h-4 rounded-full bg-[#CC0000] text-[10px] font-bold text-white flex items-center justify-center">
              3
            </span>
          </button>

          <div className="w-8 h-8 rounded-full bg-blue-600 flex items-center justify-center font-bold text-white text-xs shadow-xs ml-1 cursor-pointer">
            WP
          </div>
        </div>
      </header>

      {/* Main Body: Collapsible Sidebar + Content Feed */}
      <div className="flex-1 flex overflow-hidden">
        {/* D. Collapsible Left Sidebar (spec) */}
        {isSidebarOpen ? (
          <aside className="w-[200px] xl:w-[240px] shrink-0 border-r border-[#303030]/60 p-3 overflow-y-auto flex flex-col gap-4 text-xs font-medium bg-[#0F0F0F] transition-all">
            <div className="flex flex-col gap-0.5">
              <button
                onClick={() => {
                  setSelectedCategory('All');
                  setSearchQuery('');
                }}
                className="flex items-center gap-5 px-3 py-2.5 rounded-xl bg-[#272727] text-white font-bold transition-colors cursor-pointer"
              >
                <Home size={18} />
                <span>Home</span>
              </button>
              <button className="flex items-center gap-5 px-3 py-2.5 rounded-xl text-[#AAAAAA] hover:text-white hover:bg-[#272727] transition-colors cursor-pointer">
                <Flame size={18} />
                <span>Shorts</span>
              </button>
              <button className="flex items-center gap-5 px-3 py-2.5 rounded-xl text-[#AAAAAA] hover:text-white hover:bg-[#272727] transition-colors cursor-pointer">
                <Tv size={18} />
                <span>Subscriptions</span>
              </button>
            </div>

            <div className="h-[1px] bg-[#303030]" />

            <div className="flex flex-col gap-0.5">
              <span className="px-3 text-[11px] font-bold text-[#AAAAAA] uppercase tracking-wider mb-1">
                You
              </span>
              <button className="flex items-center gap-5 px-3 py-2.5 rounded-xl text-[#AAAAAA] hover:text-white hover:bg-[#272727] transition-colors cursor-pointer">
                <History size={18} />
                <span>History</span>
              </button>
              <button className="flex items-center gap-5 px-3 py-2.5 rounded-xl text-[#AAAAAA] hover:text-white hover:bg-[#272727] transition-colors cursor-pointer">
                <FileVideo size={18} />
                <span>Playlists</span>
              </button>
              <button className="flex items-center gap-5 px-3 py-2.5 rounded-xl text-[#AAAAAA] hover:text-white hover:bg-[#272727] transition-colors cursor-pointer">
                <Clock size={18} />
                <span>Watch later</span>
              </button>
              <button className="flex items-center gap-5 px-3 py-2.5 rounded-xl text-[#AAAAAA] hover:text-white hover:bg-[#272727] transition-colors cursor-pointer">
                <ThumbsUp size={18} />
                <span>Liked videos</span>
              </button>
            </div>

            <div className="h-[1px] bg-[#303030]" />

            <footer className="px-3 text-[10px] text-[#717171] leading-relaxed">
              <p>About Press Copyright</p>
              <p>Contact us Creators</p>
              <p className="mt-2">© 2026 YouTube Recreation inside Vibe Play</p>
            </footer>
          </aside>
        ) : (
          /* Mini Sidebar */
          <aside className="w-[72px] shrink-0 border-r border-[#303030]/60 p-1 flex flex-col items-center gap-4 text-[10px] font-normal text-[#AAAAAA] bg-[#0F0F0F]">
            <button
              onClick={() => {
                setSelectedCategory('All');
                setSearchQuery('');
              }}
              className="flex flex-col items-center gap-1.5 p-3 rounded-lg hover:bg-[#272727] hover:text-white w-full transition-colors cursor-pointer"
            >
              <Home size={18} />
              <span>Home</span>
            </button>
            <button className="flex flex-col items-center gap-1.5 p-3 rounded-lg hover:bg-[#272727] hover:text-white w-full transition-colors cursor-pointer">
              <Flame size={18} />
              <span>Shorts</span>
            </button>
            <button className="flex flex-col items-center gap-1.5 p-3 rounded-lg hover:bg-[#272727] hover:text-white w-full transition-colors cursor-pointer">
              <Tv size={18} />
              <span>Subs</span>
            </button>
            <button className="flex flex-col items-center gap-1.5 p-3 rounded-lg hover:bg-[#272727] hover:text-white w-full transition-colors cursor-pointer">
              <User size={18} />
              <span>You</span>
            </button>
          </aside>
        )}

        {/* Right Main Scrollable View */}
        <div className="flex-1 flex flex-col overflow-y-auto">
          {/* B. Category Navigation Bar (spec) */}
          <div className="sticky top-0 z-20 bg-[#0F0F0F]/95 backdrop-blur-md px-6 py-3 flex items-center gap-3 overflow-x-auto no-scrollbar border-b border-[#303030]/40 shrink-0">
            {CATEGORIES.map((cat) => {
              const isActive = selectedCategory.toLowerCase() === cat.toLowerCase();
              return (
                <button
                  key={cat}
                  onClick={() => setSelectedCategory(cat)}
                  className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold whitespace-nowrap transition-colors cursor-pointer ${
                    isActive
                      ? 'bg-white text-[#0F0F0F] shadow-sm'
                      : 'bg-[#272727] text-white hover:bg-[#3F3F3F]'
                  }`}
                >
                  {cat}
                </button>
              );
            })}
          </div>

          {/* C. Video Recommendation Grid (spec: 3 columns at desktop) */}
          <main className="flex-1 p-6">
            {filteredVideos.length === 0 ? (
              <div className="h-64 flex flex-col items-center justify-center text-center text-[#AAAAAA]">
                <Search size={36} className="mb-2 text-[#717171]" />
                <p className="text-sm font-semibold">No videos found matching &ldquo;{searchQuery}&rdquo;</p>
                <button
                  onClick={() => {
                    setSearchQuery('');
                    setSelectedCategory('All');
                  }}
                  className="mt-3 text-xs text-[#3EA6FF] hover:underline cursor-pointer"
                >
                  Clear search and filters
                </button>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-4 gap-y-8">
                {filteredVideos.map((video) => (
                  <div
                    key={video.id}
                    onClick={() => setActiveVideo(video)}
                    className="flex flex-col gap-3 group cursor-pointer"
                  >
                    {/* Video Thumbnail (16:9, rounded corners 10-12px per spec) */}
                    <div
                      className="relative aspect-video w-full rounded-[11px] overflow-hidden bg-zinc-900 shadow-md transition-transform duration-200 group-hover:scale-[1.01] flex items-center justify-center border border-[#303030]/50"
                      style={{ background: video.thumbnailGradient }}
                    >
                      <span className="px-2.5 py-1 rounded text-[11px] font-bold tracking-wider uppercase text-white bg-black/40 border border-white/20 text-center max-w-[85%] truncate drop-shadow-sm">
                        {video.thumbnailTag}
                      </span>

                      {/* Duration Badge Bottom-Right */}
                      <span
                        className={`absolute bottom-2 right-2 px-1.5 py-0.5 rounded text-[11px] font-semibold text-white shadow-md ${
                          video.duration === 'LIVE' ? 'bg-[#CC0000]' : 'bg-black/85'
                        }`}
                      >
                        {video.duration}
                      </span>
                    </div>

                    {/* Video Information Below Thumbnail (spec) */}
                    <div className="flex gap-3 px-0.5">
                      {/* Circular Creator Avatar */}
                      <div
                        className="w-9 h-9 rounded-full flex items-center justify-center font-bold text-white text-xs shrink-0 shadow-sm"
                        style={{ backgroundColor: video.channelAvatarBg }}
                      >
                        {video.channelAvatarText}
                      </div>

                      {/* Video Title & Metadata */}
                      <div className="flex-1 flex flex-col">
                        <h3 className="text-sm font-bold text-[#F1F1F1] group-hover:text-[#3EA6FF] line-clamp-2 leading-snug transition-colors">
                          {video.title}
                        </h3>

                        <div className="flex items-center gap-1.5 mt-1 text-xs text-[#AAAAAA]">
                          <span className="hover:text-white transition-colors">{video.channel}</span>
                          {video.isVerified && (
                            <CheckCircle2 size={12} className="text-[#AAAAAA] fill-[#AAAAAA]" />
                          )}
                        </div>

                        <div className="text-xs text-[#AAAAAA]">
                          <span>{video.views}</span>
                          <span className="mx-1">•</span>
                          <span>{video.uploadTime}</span>
                        </div>
                      </div>

                      {/* Three-Dot Options Menu */}
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                        }}
                        className="p-1 text-[#AAAAAA] hover:text-white opacity-0 group-hover:opacity-100 transition-opacity self-start"
                      >
                        <MoreVertical size={16} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </main>
        </div>
      </div>
    </div>
  );
};
