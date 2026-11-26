import React, { useState } from "react";
import {
  Menu,
  Search,
  Mic,
  Video as VideoIcon,
  Bell,
  Home,
  Flame,
  Tv,
  FolderMinus,
  History,
  Clock,
  ThumbsUp,
  MoreVertical,
  CheckCircle2,
  Share2,
  Bookmark,
  X,
  PlaySquare,
} from "lucide-react";
import {
  YouTubeVideo,
  YOUTUBE_CATEGORIES,
  INITIAL_YOUTUBE_VIDEOS,
} from "./YouTubeTypes";
import { YouTubeWatchView } from "./YouTubeWatchView";

interface YouTubeContainerProps {
  onClose?: () => void;
}

export const YouTubeContainer: React.FC<YouTubeContainerProps> = ({ onClose }) => {
  const [videos, setVideos] = useState<YouTubeVideo[]>(INITIAL_YOUTUBE_VIDEOS);
  const [selectedCategory, setSelectedCategory] = useState("All");
  const [searchQuery, setSearchQuery] = useState("");
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);
  const [activeVideo, setActiveVideo] = useState<YouTubeVideo | null>(null);
  const [optionsVideoId, setOptionsVideoId] = useState<string | null>(null);

  // Filter videos based on category and search query
  const filteredVideos = videos.filter((v) => {
    const matchesCategory =
      selectedCategory === "All" || v.category.toLowerCase() === selectedCategory.toLowerCase();
    const matchesSearch =
      searchQuery.trim() === "" ||
      v.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      v.channel.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      v.category.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  return (
    <div className="w-full h-full flex flex-col bg-[#0F0F0F] text-[#F1F1F1] overflow-hidden select-none font-sans">
      {/* ========================================================================= */}
      {/* 2.A TOP NAVIGATION BAR                                                   */}
      {/* ========================================================================= */}
      <header className="h-14 px-4 border-b border-[#303030] bg-[#0F0F0F] flex items-center justify-between shrink-0 z-30">
        {/* Left Section: Hamburger + YouTube Wordmark */}
        <div className="flex items-center gap-4">
          <button
            onClick={() => setIsSidebarOpen(!isSidebarOpen)}
            className="p-2 rounded-full hover:bg-[#272727] text-white transition-colors cursor-pointer"
            title="Toggle Sidebar"
          >
            <Menu className="w-5 h-5" />
          </button>

          {/* YouTube Brand Logo */}
          <div
            onClick={() => setActiveVideo(null)}
            className="flex items-center gap-1.5 cursor-pointer group"
          >
            {/* Red Play Pill */}
            <div className="w-7 h-5 rounded-md bg-[#FF0000] flex items-center justify-center text-white shadow-xs">
              <div className="w-0 h-0 border-y-[4px] border-y-transparent border-l-[7px] border-l-white translate-x-[1px]" />
            </div>
            <span className="text-lg font-bold tracking-tighter text-white font-sans">
              YouTube
            </span>
            <span className="text-[10px] text-zinc-400 font-semibold uppercase -translate-y-2">
              Vibe Play
            </span>
          </div>
        </div>

        {/* Center Section: Search Bar & Mic */}
        <div className="flex-1 max-w-2xl mx-4 hidden sm:flex items-center justify-center gap-2">
          <div className="flex items-center w-full max-w-lg bg-[#121212] border border-[#303030] rounded-full overflow-hidden focus-within:border-blue-500">
            <input
              type="text"
              placeholder="Search"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="flex-1 bg-transparent px-4 py-2 text-sm text-white placeholder-[#AAAAAA] focus:outline-hidden"
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery("")}
                className="px-2 text-zinc-400 hover:text-white"
              >
                <X className="w-4 h-4" />
              </button>
            )}
            <button
              className="bg-[#222222] hover:bg-[#272727] px-5 py-2 border-l border-[#303030] text-[#AAAAAA] hover:text-white transition-colors cursor-pointer"
              title="Search"
            >
              <Search className="w-4 h-4" />
            </button>
          </div>

          {/* Microphone button beside search */}
          <button
            className="p-2.5 rounded-full bg-[#222222] hover:bg-[#272727] text-white transition-colors cursor-pointer"
            title="Search with your voice"
          >
            <Mic className="w-4 h-4" />
          </button>
        </div>

        {/* Right Section: Create, Notifications, Profile Avatar, Optional Close */}
        <div className="flex items-center gap-2">
          <button
            className="hidden md:flex items-center gap-2 px-3 py-1.5 rounded-full bg-[#272727] hover:bg-[#3F3F3F] text-xs font-semibold text-white transition-colors cursor-pointer"
            title="Create"
          >
            <VideoIcon className="w-4 h-4" />
            <span>Create</span>
          </button>

          <button
            className="p-2 rounded-full hover:bg-[#272727] text-white relative transition-colors cursor-pointer"
            title="Notifications"
          >
            <Bell className="w-5 h-5" />
            <span className="absolute top-1.5 right-1.5 w-2 h-2 rounded-full bg-red-600" />
          </button>

          <div
            className="w-8 h-8 rounded-full bg-blue-600 text-white text-xs font-bold flex items-center justify-center shadow-md cursor-pointer"
            title="Account"
          >
            WP
          </div>

          {onClose && (
            <button
              onClick={onClose}
              className="ml-2 p-1.5 rounded-lg text-zinc-400 hover:text-white hover:bg-zinc-800 transition-colors"
              title="Close"
            >
              <X className="w-5 h-5" />
            </button>
          )}
        </div>
      </header>

      {/* ========================================================================= */}
      {/* BODY: SIDEBAR + MAIN CONTENT AREA (HOMEPAGE OR WATCH VIEW)                */}
      {/* ========================================================================= */}
      <div className="flex-1 flex min-h-0 overflow-hidden relative">
        {/* 2.D COLLAPSIBLE SIDEBAR */}
        <aside
          className={`${
            isSidebarOpen ? "hidden md:flex md:w-56 px-3" : "hidden sm:flex sm:w-18 px-1"
          } py-3 border-r border-[#303030] bg-[#0F0F0F] flex-col justify-between shrink-0 transition-all duration-200 overflow-y-auto no-scrollbar z-20`}
        >
          <div className="space-y-1">
            <button
              onClick={() => setActiveVideo(null)}
              className={`w-full flex items-center gap-4 px-3 py-2.5 rounded-xl text-xs font-medium cursor-pointer transition-colors ${
                !activeVideo
                  ? "bg-[#272727] text-white font-semibold"
                  : "text-zinc-300 hover:bg-[#222222] hover:text-white"
              }`}
            >
              <Home className="w-5 h-5 shrink-0" />
              {isSidebarOpen && <span>Home</span>}
            </button>

            <button className="w-full flex items-center gap-4 px-3 py-2.5 rounded-xl text-xs font-medium text-zinc-300 hover:bg-[#222222] hover:text-white transition-colors cursor-pointer">
              <Flame className="w-5 h-5 shrink-0" />
              {isSidebarOpen && <span>Shorts</span>}
            </button>

            <button className="w-full flex items-center gap-4 px-3 py-2.5 rounded-xl text-xs font-medium text-zinc-300 hover:bg-[#222222] hover:text-white transition-colors cursor-pointer">
              <Tv className="w-5 h-5 shrink-0" />
              {isSidebarOpen && <span>Subscriptions</span>}
            </button>

            <div className="my-2 border-t border-[#303030]" />

            <div className="px-3 py-1.5 text-[11px] font-semibold text-[#AAAAAA]">
              {isSidebarOpen ? "You" : "•••"}
            </div>

            <button className="w-full flex items-center gap-4 px-3 py-2.5 rounded-xl text-xs font-medium text-zinc-300 hover:bg-[#222222] hover:text-white transition-colors cursor-pointer">
              <History className="w-5 h-5 shrink-0" />
              {isSidebarOpen && <span>History</span>}
            </button>

            <button className="w-full flex items-center gap-4 px-3 py-2.5 rounded-xl text-xs font-medium text-zinc-300 hover:bg-[#222222] hover:text-white transition-colors cursor-pointer">
              <PlaySquare className="w-5 h-5 shrink-0" />
              {isSidebarOpen && <span>Playlists</span>}
            </button>

            <button className="w-full flex items-center gap-4 px-3 py-2.5 rounded-xl text-xs font-medium text-zinc-300 hover:bg-[#222222] hover:text-white transition-colors cursor-pointer">
              <Clock className="w-5 h-5 shrink-0" />
              {isSidebarOpen && <span>Watch later</span>}
            </button>

            <button className="w-full flex items-center gap-4 px-3 py-2.5 rounded-xl text-xs font-medium text-zinc-300 hover:bg-[#222222] hover:text-white transition-colors cursor-pointer">
              <ThumbsUp className="w-5 h-5 shrink-0" />
              {isSidebarOpen && <span>Liked videos</span>}
            </button>
          </div>

          {isSidebarOpen && (
            <div className="text-[11px] text-zinc-500 pt-4 px-3 border-t border-[#303030]">
              <p>About Press Copyright</p>
              <p className="mt-1">Contact us Creators</p>
              <p className="mt-2 text-[10px]">© 2026 Google LLC</p>
            </div>
          )}
        </aside>

        {/* MAIN BODY: WATCH VIEW OR HOMEPAGE GRID */}
        {activeVideo ? (
          <YouTubeWatchView
            video={activeVideo}
            suggestedVideos={videos}
            onSelectVideo={(v) => setActiveVideo(v)}
            onBackToHome={() => setActiveVideo(null)}
          />
        ) : (
          <div className="flex-1 flex flex-col min-w-0 overflow-y-auto">
            {/* 2.B CATEGORY NAVIGATION CHIPS */}
            <div className="sticky top-0 z-10 bg-[#0F0F0F]/95 backdrop-blur-md px-6 py-3 border-b border-[#303030]/60 flex items-center gap-2.5 overflow-x-auto no-scrollbar">
              {YOUTUBE_CATEGORIES.map((cat) => {
                const isActive = selectedCategory === cat;
                return (
                  <button
                    key={cat}
                    onClick={() => setSelectedCategory(cat)}
                    className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold shrink-0 transition-colors cursor-pointer ${
                      isActive
                        ? "bg-[#F1F1F1] text-[#0F0F0F]"
                        : "bg-[#272727] text-[#F1F1F1] hover:bg-[#3F3F3F]"
                    }`}
                  >
                    {cat}
                  </button>
                );
              })}
            </div>

            {/* 2.C VIDEO RECOMMENDATION GRID */}
            <div className="p-6 md:p-8">
              {filteredVideos.length === 0 ? (
                <div className="py-20 text-center space-y-2">
                  <p className="text-base font-semibold text-white">No videos found</p>
                  <p className="text-xs text-zinc-400">
                    Try searching for another topic or selecting "All"
                  </p>
                  <button
                    onClick={() => {
                      setSearchQuery("");
                      setSelectedCategory("All");
                    }}
                    className="mt-2 px-4 py-2 rounded-full bg-[#272727] hover:bg-[#3F3F3F] text-xs font-semibold text-white"
                  >
                    Reset Filters
                  </button>
                </div>
              ) : (
                /* Three columns at 1080p desktop width, 2 on smaller, 1 on mobile */
                <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-x-6 gap-y-10 max-w-[1680px] mx-auto">
                  {filteredVideos.map((video) => (
                    <article
                      key={video.id}
                      className="group flex flex-col space-y-3 cursor-pointer"
                      onClick={() => setActiveVideo(video)}
                    >
                      {/* 16:9 Thumbnail Container */}
                      <div className="relative aspect-video w-full rounded-[11px] overflow-hidden bg-zinc-900 border border-[#303030]/40 shadow-sm group-hover:shadow-md transition-shadow">
                        <img
                          src={video.thumbnailUrl}
                          alt={video.title}
                          className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                        />

                        {/* Optional thumbnail tag badge */}
                        {video.badge && (
                          <div className="absolute top-2 left-2 px-2 py-0.5 rounded bg-black/80 backdrop-blur-xs text-[10px] font-bold text-white tracking-wider">
                            {video.badge}
                          </div>
                        )}

                        {/* Duration Badge Bottom-Right */}
                        <div className="absolute bottom-2 right-2 px-1.5 py-0.5 rounded bg-black/80 text-[11px] font-semibold text-white font-mono">
                          {video.duration}
                        </div>
                      </div>

                      {/* Video Info: Creator Avatar + Title + Channel + Metadata */}
                      <div className="flex items-start gap-3 relative">
                        <img
                          src={video.channel.avatar}
                          alt={video.channel.name}
                          className="w-9 h-9 rounded-full object-cover shrink-0 mt-0.5 border border-[#303030]"
                        />

                        <div className="flex-1 min-w-0">
                          {/* Title */}
                          <h3 className="text-sm font-semibold text-[#F1F1F1] line-clamp-2 leading-snug group-hover:text-blue-400 transition-colors">
                            {video.title}
                          </h3>

                          {/* Channel Name */}
                          <div className="mt-1 flex items-center gap-1.5 text-xs text-[#AAAAAA]">
                            <span className="hover:text-white transition-colors">
                              {video.channel.name}
                            </span>
                            {video.channel.verified && (
                              <CheckCircle2 className="w-3.5 h-3.5 text-[#AAAAAA] fill-[#AAAAAA]/20" />
                            )}
                          </div>

                          {/* Views & Uploaded Time */}
                          <div className="text-xs text-[#AAAAAA]">
                            <span>{video.views}</span>
                            <span className="mx-1">•</span>
                            <span>{video.uploadedAt}</span>
                          </div>
                        </div>

                        {/* 3-Dot Options Menu */}
                        <div className="relative">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setOptionsVideoId(
                                optionsVideoId === video.id ? null : video.id
                              );
                            }}
                            className="p-1 rounded-full text-transparent group-hover:text-[#AAAAAA] hover:text-white transition-colors cursor-pointer"
                            title="Action menu"
                          >
                            <MoreVertical className="w-4 h-4" />
                          </button>

                          {/* Options Popup Dropdown */}
                          {optionsVideoId === video.id && (
                            <div
                              onClick={(e) => e.stopPropagation()}
                              className="absolute right-0 top-7 z-30 w-48 bg-[#272727] border border-[#303030] rounded-xl shadow-2xl py-1 text-xs text-white"
                            >
                              <button
                                onClick={() => {
                                  setActiveVideo(video);
                                  setOptionsVideoId(null);
                                }}
                                className="w-full text-left px-3 py-2 hover:bg-[#3F3F3F] flex items-center gap-2"
                              >
                                <PlaySquare className="w-4 h-4 text-zinc-300" />
                                <span>Play Now</span>
                              </button>
                              <button
                                onClick={() => setOptionsVideoId(null)}
                                className="w-full text-left px-3 py-2 hover:bg-[#3F3F3F] flex items-center gap-2"
                              >
                                <Clock className="w-4 h-4 text-zinc-300" />
                                <span>Save to Watch Later</span>
                              </button>
                              <button
                                onClick={() => setOptionsVideoId(null)}
                                className="w-full text-left px-3 py-2 hover:bg-[#3F3F3F] flex items-center gap-2"
                              >
                                <Share2 className="w-4 h-4 text-zinc-300" />
                                <span>Share Video</span>
                              </button>
                            </div>
                          )}
                        </div>
                      </div>
                    </article>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
