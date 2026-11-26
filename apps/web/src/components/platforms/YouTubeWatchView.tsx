import React, { useState } from "react";
import {
  Play,
  Pause,
  Volume2,
  VolumeX,
  Maximize2,
  ThumbsUp,
  ThumbsDown,
  Share2,
  Download,
  MoreHorizontal,
  CheckCircle2,
  ArrowLeft,
  MessageSquare,
} from "lucide-react";
import { YouTubeVideo } from "./YouTubeTypes";

interface YouTubeWatchViewProps {
  video: YouTubeVideo;
  suggestedVideos: YouTubeVideo[];
  onSelectVideo: (video: YouTubeVideo) => void;
  onBackToHome: () => void;
}

export const YouTubeWatchView: React.FC<YouTubeWatchViewProps> = ({
  video,
  suggestedVideos,
  onSelectVideo,
  onBackToHome,
}) => {
  const [isPlaying, setIsPlaying] = useState(true);
  const [isMuted, setIsMuted] = useState(false);
  const [isLiked, setIsLiked] = useState(false);
  const [isDisliked, setIsDisliked] = useState(false);
  const [isSubscribed, setIsSubscribed] = useState(video.channel.isSubscribed || false);
  const [isDescExpanded, setIsDescExpanded] = useState(false);
  const [commentInput, setCommentInput] = useState("");
  const [comments, setComments] = useState(video.comments);
  const [progressSec, setProgressSec] = useState(38);

  const toggleLike = () => {
    if (isLiked) {
      setIsLiked(false);
    } else {
      setIsLiked(true);
      setIsDisliked(false);
    }
  };

  const toggleDislike = () => {
    if (isDisliked) {
      setIsDisliked(false);
    } else {
      setIsDisliked(true);
      setIsLiked(false);
    }
  };

  const handleAddComment = () => {
    if (!commentInput.trim()) return;
    setComments([
      {
        author: "You (Vibe Play)",
        avatar: "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=150&auto=format&fit=crop&q=80",
        text: commentInput.trim(),
        timeAgo: "Just now",
        likes: 0,
      },
      ...comments,
    ]);
    setCommentInput("");
  };

  return (
    <div className="flex-1 bg-[#0F0F0F] text-[#F1F1F1] overflow-y-auto px-4 md:px-8 py-6 font-sans">
      {/* Back to Home Button */}
      <button
        onClick={onBackToHome}
        className="mb-4 inline-flex items-center gap-2 px-3 py-1.5 rounded-lg bg-[#272727] hover:bg-[#3F3F3F] text-xs font-semibold text-white transition-colors cursor-pointer"
      >
        <ArrowLeft className="w-4 h-4" />
        <span>Back to YouTube Home</span>
      </button>

      <div className="max-w-[1720px] mx-auto grid grid-cols-1 lg:grid-cols-3 xl:grid-cols-4 gap-8">
        {/* Left 2/3 Column: Video Player & Metadata */}
        <div className="lg:col-span-2 xl:col-span-3 space-y-4">
          {/* Main 16:9 Video Player Container */}
          <div className="relative aspect-video w-full bg-black rounded-xl overflow-hidden shadow-2xl border border-[#303030] group select-none">
            {/* Poster / Content */}
            <div
              className="absolute inset-0 bg-cover bg-center transition-all duration-300"
              style={{
                backgroundImage: `url(${video.thumbnailUrl})`,
                filter: isPlaying ? "none" : "brightness(0.6)",
              }}
            >
              <div className="absolute inset-0 bg-black/30" />
            </div>

            {/* Ambient Player Branding Display */}
            <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
              <div className="p-4 rounded-2xl bg-black/60 backdrop-blur-md border border-white/10 text-center animate-pulse">
                <div className="flex items-center justify-center gap-2 text-white font-bold text-lg">
                  <span className="w-3 h-3 rounded-full bg-red-600 inline-block animate-ping" />
                  <span>Now Playing: {video.title.slice(0, 35)}...</span>
                </div>
                <p className="text-xs text-zinc-300 mt-1">
                  Vibe Play Live Player • Streaming while agent reasons
                </p>
              </div>
            </div>

            {/* Player Controls Bar (YouTube Dark Style) */}
            <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/90 via-black/50 to-transparent p-4 flex flex-col gap-2 z-20">
              {/* Timeline Scrubber */}
              <div className="relative w-full h-1 group-hover:h-2 bg-white/20 rounded-full cursor-pointer transition-all">
                <div
                  className="h-full bg-red-600 rounded-full relative"
                  style={{ width: "35%" }}
                >
                  <div className="absolute right-0 top-1/2 -translate-y-1/2 w-3 h-3 bg-red-600 rounded-full shadow-md opacity-0 group-hover:opacity-100 transition-opacity" />
                </div>
              </div>

              {/* Action Buttons Row */}
              <div className="flex items-center justify-between text-white text-xs pt-1">
                <div className="flex items-center gap-4">
                  <button
                    onClick={() => setIsPlaying(!isPlaying)}
                    className="p-1.5 hover:text-red-500 transition-colors cursor-pointer"
                    title={isPlaying ? "Pause" : "Play"}
                  >
                    {isPlaying ? <Pause className="w-5 h-5" /> : <Play className="w-5 h-5 fill-white" />}
                  </button>

                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => setIsMuted(!isMuted)}
                      className="p-1 hover:text-zinc-300 transition-colors cursor-pointer"
                    >
                      {isMuted ? <VolumeX className="w-5 h-5" /> : <Volume2 className="w-5 h-5" />}
                    </button>
                    <div className="w-16 h-1 bg-white/30 rounded-full overflow-hidden">
                      <div className="h-full bg-white w-3/4 rounded-full" />
                    </div>
                  </div>

                  <span className="text-[11px] text-zinc-300 font-mono">
                    04:12 / {video.duration}
                  </span>
                </div>

                <div className="flex items-center gap-3">
                  <button
                    className="p-1 hover:text-zinc-300 cursor-pointer"
                    title="Fullscreen"
                  >
                    <Maximize2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          </div>

          {/* Video Title */}
          <h1 className="text-xl md:text-2xl font-bold tracking-tight text-[#F1F1F1] leading-snug">
            {video.title}
          </h1>

          {/* Channel Row & Engagement Actions */}
          <div className="flex flex-wrap items-center justify-between gap-4 py-2 border-b border-[#303030]">
            {/* Channel Info */}
            <div className="flex items-center gap-3">
              <img
                src={video.channel.avatar}
                alt={video.channel.name}
                className="w-11 h-11 rounded-full object-cover border border-[#303030]"
              />
              <div>
                <div className="flex items-center gap-1.5">
                  <span className="font-semibold text-sm text-white hover:underline cursor-pointer">
                    {video.channel.name}
                  </span>
                  {video.channel.verified && (
                    <CheckCircle2 className="w-3.5 h-3.5 text-[#AAAAAA] fill-[#AAAAAA]/20" />
                  )}
                </div>
                <p className="text-xs text-[#AAAAAA]">{video.channel.subscribers}</p>
              </div>

              <button
                onClick={() => setIsSubscribed(!isSubscribed)}
                className={`ml-3 px-4 py-2 rounded-full text-xs font-semibold transition-colors cursor-pointer ${
                  isSubscribed
                    ? "bg-[#272727] text-white hover:bg-[#3F3F3F]"
                    : "bg-white text-black hover:bg-zinc-200"
                }`}
              >
                {isSubscribed ? "Subscribed" : "Subscribe"}
              </button>
            </div>

            {/* Action Buttons (Like/Dislike Pill, Share, Download) */}
            <div className="flex items-center gap-2">
              {/* Like / Dislike Combined Pill */}
              <div className="flex items-center bg-[#272727] rounded-full overflow-hidden border border-transparent hover:border-[#3F3F3F]">
                <button
                  onClick={toggleLike}
                  className={`flex items-center gap-2 px-3.5 py-2 text-xs font-semibold transition-colors cursor-pointer ${
                    isLiked ? "text-blue-400" : "text-white hover:bg-[#3F3F3F]"
                  }`}
                >
                  <ThumbsUp className={`w-4 h-4 ${isLiked ? "fill-blue-400" : ""}`} />
                  <span>{video.likes}</span>
                </button>
                <div className="w-px h-5 bg-[#3F3F3F]" />
                <button
                  onClick={toggleDislike}
                  className={`px-3 py-2 text-xs transition-colors cursor-pointer ${
                    isDisliked ? "text-blue-400" : "text-white hover:bg-[#3F3F3F]"
                  }`}
                >
                  <ThumbsDown className={`w-4 h-4 ${isDisliked ? "fill-blue-400" : ""}`} />
                </button>
              </div>

              {/* Share */}
              <button className="flex items-center gap-2 px-3.5 py-2 rounded-full bg-[#272727] hover:bg-[#3F3F3F] text-xs font-semibold text-white transition-colors cursor-pointer">
                <Share2 className="w-4 h-4" />
                <span>Share</span>
              </button>

              {/* Download */}
              <button className="hidden sm:flex items-center gap-2 px-3.5 py-2 rounded-full bg-[#272727] hover:bg-[#3F3F3F] text-xs font-semibold text-white transition-colors cursor-pointer">
                <Download className="w-4 h-4" />
                <span>Download</span>
              </button>

              {/* More */}
              <button className="p-2 rounded-full bg-[#272727] hover:bg-[#3F3F3F] text-white transition-colors cursor-pointer">
                <MoreHorizontal className="w-4 h-4" />
              </button>
            </div>
          </div>

          {/* Description Box (YouTube Dark Expandable Container) */}
          <div
            onClick={() => setIsDescExpanded(!isDescExpanded)}
            className="p-3.5 rounded-xl bg-[#272727]/70 hover:bg-[#272727] transition-colors cursor-pointer text-xs space-y-1.5"
          >
            <div className="font-semibold text-white flex items-center gap-2">
              <span>{video.views}</span>
              <span>•</span>
              <span>{video.uploadedAt}</span>
              {video.badge && (
                <span className="px-1.5 py-0.5 rounded bg-blue-900/60 text-blue-300 text-[10px] font-bold">
                  {video.badge}
                </span>
              )}
            </div>
            <p className="text-zinc-300 leading-relaxed">
              {isDescExpanded
                ? video.description
                : `${video.description.slice(0, 140)}... Show more`}
            </p>
          </div>

          {/* Comments Section */}
          <div className="pt-4 space-y-5">
            <div className="flex items-center gap-6">
              <h3 className="text-base font-bold text-white">
                {video.commentsCount} Comments
              </h3>
              <button className="text-xs font-semibold text-[#AAAAAA] hover:text-white flex items-center gap-1.5 cursor-pointer">
                <MessageSquare className="w-4 h-4" />
                <span>Sort by</span>
              </button>
            </div>

            {/* Add Comment Input Row */}
            <div className="flex items-start gap-3">
              <div className="w-10 h-10 rounded-full bg-blue-600 text-white font-bold flex items-center justify-center shrink-0">
                WP
              </div>
              <div className="flex-1 space-y-2">
                <input
                  type="text"
                  placeholder="Add a comment..."
                  value={commentInput}
                  onChange={(e) => setCommentInput(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleAddComment()}
                  className="w-full bg-transparent border-b border-[#303030] focus:border-white py-1.5 text-xs text-white placeholder-[#AAAAAA] focus:outline-hidden"
                />
                {commentInput.trim() && (
                  <div className="flex justify-end gap-2">
                    <button
                      onClick={() => setCommentInput("")}
                      className="px-3 py-1.5 rounded-full text-xs font-semibold text-[#AAAAAA] hover:text-white"
                    >
                      Cancel
                    </button>
                    <button
                      onClick={handleAddComment}
                      className="px-4 py-1.5 rounded-full bg-blue-600 hover:bg-blue-500 text-xs font-semibold text-white"
                    >
                      Comment
                    </button>
                  </div>
                )}
              </div>
            </div>

            {/* Comments List */}
            <div className="space-y-4 pt-2">
              {comments.map((c, i) => (
                <div key={i} className="flex items-start gap-3 text-xs">
                  <img
                    src={c.avatar}
                    alt={c.author}
                    className="w-9 h-9 rounded-full object-cover shrink-0"
                  />
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-white">{c.author}</span>
                      <span className="text-[11px] text-[#AAAAAA]">{c.timeAgo}</span>
                    </div>
                    <p className="text-zinc-200 leading-relaxed">{c.text}</p>
                    <div className="flex items-center gap-3 text-[11px] text-[#AAAAAA] pt-1">
                      <button className="flex items-center gap-1 hover:text-white">
                        <ThumbsUp className="w-3.5 h-3.5" />
                        <span>{c.likes > 0 ? c.likes : ""}</span>
                      </button>
                      <button className="hover:text-white">
                        <ThumbsDown className="w-3.5 h-3.5" />
                      </button>
                      <button className="font-semibold hover:text-white">Reply</button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Right 1/3 Column: Up Next Recommended Videos */}
        <aside className="space-y-4">
          <h4 className="text-sm font-bold text-white tracking-tight">
            Up next for your break
          </h4>

          <div className="flex flex-col space-y-3">
            {suggestedVideos
              .filter((v) => v.id !== video.id)
              .map((item) => (
                <div
                  key={item.id}
                  onClick={() => onSelectVideo(item)}
                  className="flex gap-3 group cursor-pointer p-1.5 rounded-xl hover:bg-[#272727]/60 transition-colors"
                >
                  {/* Small 16:9 Thumbnail */}
                  <div className="relative w-40 aspect-video rounded-lg overflow-hidden shrink-0 bg-black">
                    <img
                      src={item.thumbnailUrl}
                      alt={item.title}
                      className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-200"
                    />
                    <span className="absolute bottom-1 right-1 px-1.5 py-0.5 rounded bg-black/80 text-[10px] font-semibold text-white">
                      {item.duration}
                    </span>
                  </div>

                  {/* Metadata */}
                  <div className="flex flex-col justify-start">
                    <h5 className="text-xs font-semibold text-white line-clamp-2 leading-snug group-hover:text-blue-400 transition-colors">
                      {item.title}
                    </h5>
                    <div className="mt-1 flex items-center gap-1 text-[11px] text-[#AAAAAA]">
                      <span>{item.channel.name}</span>
                      {item.channel.verified && (
                        <CheckCircle2 className="w-3 h-3 text-[#AAAAAA]" />
                      )}
                    </div>
                    <div className="text-[10px] text-[#AAAAAA] mt-0.5">
                      <span>{item.views}</span> • <span>{item.uploadedAt}</span>
                    </div>
                  </div>
                </div>
              ))}
          </div>
        </aside>
      </div>
    </div>
  );
};
