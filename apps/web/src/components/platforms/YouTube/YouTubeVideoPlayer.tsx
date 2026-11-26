import React, { useState, useEffect } from 'react';
import {
  Play,
  Pause,
  Volume2,
  VolumeX,
  Maximize2,
  Minimize2,
  ThumbsUp,
  ThumbsDown,
  Share2,
  Download,
  Bookmark,
  MoreHorizontal,
  ArrowLeft,
  CheckCircle2,
  Sparkles,
  RotateCcw,
} from 'lucide-react';

export interface YouTubeVideoData {
  id: string;
  title: string;
  category: string;
  duration: string;
  views: string;
  uploadTime: string;
  channel: string;
  subscribers: string;
  channelAvatarText: string;
  channelAvatarBg: string;
  isVerified: boolean;
  thumbnailGradient: string;
  thumbnailTag: string;
  description: string;
}

interface YouTubeVideoPlayerProps {
  video: YouTubeVideoData;
  onBack: () => void;
  onSelectRelated: (video: YouTubeVideoData) => void;
  relatedVideos: YouTubeVideoData[];
}

export const YouTubeVideoPlayer: React.FC<YouTubeVideoPlayerProps> = ({
  video,
  onBack,
  onSelectRelated,
  relatedVideos,
}) => {
  const [isPlaying, setIsPlaying] = useState(true);
  const [isMuted, setIsMuted] = useState(false);
  const [progressSec, setProgressSec] = useState(45);
  const [isSubscribed, setIsSubscribed] = useState(false);
  const [isLiked, setIsLiked] = useState(false);
  const [likesCount, setLikesCount] = useState(14200);
  const [isDisliked, setIsDisliked] = useState(false);
  const [isDescExpanded, setIsDescExpanded] = useState(false);

  const totalDurationSec = 780; // 13:00

  useEffect(() => {
    let timer: number;
    if (isPlaying) {
      timer = window.setInterval(() => {
        setProgressSec((prev) => (prev >= totalDurationSec ? 0 : prev + 1));
      }, 1000);
    }
    return () => clearInterval(timer);
  }, [isPlaying, totalDurationSec]);

  const formatTime = (sec: number) => {
    const mins = Math.floor(sec / 60);
    const remainder = sec % 60;
    return `${mins}:${remainder.toString().padStart(2, '0')}`;
  };

  const toggleLike = () => {
    if (isLiked) {
      setIsLiked(false);
      setLikesCount((c) => c - 1);
    } else {
      setIsLiked(true);
      setLikesCount((c) => c + 1);
      setIsDisliked(false);
    }
  };

  const toggleDislike = () => {
    setIsDisliked(!isDisliked);
    if (!isDisliked && isLiked) {
      setIsLiked(false);
      setLikesCount((c) => c - 1);
    }
  };

  const progressPercent = (progressSec / totalDurationSec) * 100;

  return (
    <div className="w-full h-full bg-[#0F0F0F] text-[#F1F1F1] overflow-y-auto font-sans select-none">
      {/* Top Header Bar */}
      <div className="sticky top-0 z-40 bg-[#0F0F0F]/90 backdrop-blur-md px-6 py-3 border-b border-[#303030] flex items-center justify-between">
        <button
          onClick={onBack}
          className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-800 hover:bg-zinc-700 text-xs font-semibold text-white transition-colors cursor-pointer"
        >
          <ArrowLeft size={16} />
          <span>Back to YouTube</span>
        </button>

        <span className="text-xs text-[#AAAAAA] truncate max-w-md hidden sm:block">
          {video.title}
        </span>
      </div>

      {/* Main Watch Layout: Video & Info + Related Column */}
      <div className="max-w-[1700px] mx-auto p-4 lg:p-8 flex flex-col lg:flex-row gap-8">
        {/* Left Primary Column: Player + Video Info */}
        <div className="flex-1 flex flex-col gap-4">
          {/* 16:9 Video Player Box */}
          <div className="relative aspect-video w-full rounded-2xl overflow-hidden bg-black shadow-2xl border border-[#303030] group">
            {/* Visual Screen Canvas */}
            <div
              onClick={() => setIsPlaying(!isPlaying)}
              className="absolute inset-0 cursor-pointer flex flex-col items-center justify-center p-8 text-center"
              style={{ background: video.thumbnailGradient }}
            >
              <span className="px-3 py-1 rounded-full text-xs font-bold tracking-widest uppercase mb-4 bg-white/10 text-white border border-white/20">
                {video.thumbnailTag}
              </span>
              <h1 className="text-2xl md:text-3xl font-black text-white max-w-xl leading-snug drop-shadow-xl">
                {video.title}
              </h1>
              <p className="text-xs text-white/80 mt-2">
                Click video to {isPlaying ? 'pause' : 'resume'} • Vibe Play Companion Mode
              </p>
            </div>

            {/* Bottom Controls Overlay */}
            <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/90 via-black/50 to-transparent p-4 flex flex-col gap-2.5 transition-opacity duration-200">
              {/* Red Progress Bar */}
              <div
                onClick={(e) => {
                  const rect = e.currentTarget.getBoundingClientRect();
                  const fraction = (e.clientX - rect.left) / rect.width;
                  setProgressSec(Math.floor(fraction * totalDurationSec));
                }}
                className="w-full h-1.5 hover:h-2.5 bg-white/30 rounded-full cursor-pointer relative group/bar transition-all"
              >
                <div
                  className="h-full bg-[#FF0000] rounded-full relative"
                  style={{ width: `${progressPercent}%` }}
                >
                  <div className="absolute right-0 top-1/2 -translate-y-1/2 w-3.5 h-3.5 rounded-full bg-[#FF0000] shadow-md opacity-0 group-hover/bar:opacity-100 transition-opacity" />
                </div>
              </div>

              {/* Control Buttons */}
              <div className="flex items-center justify-between text-white text-xs">
                <div className="flex items-center gap-4">
                  <button
                    onClick={() => setIsPlaying(!isPlaying)}
                    className="hover:scale-110 active:scale-95 transition-transform cursor-pointer"
                  >
                    {isPlaying ? <Pause size={20} /> : <Play size={20} />}
                  </button>
                  <button
                    onClick={() => setIsMuted(!isMuted)}
                    className="hover:scale-110 active:scale-95 transition-transform cursor-pointer"
                  >
                    {isMuted ? <VolumeX size={18} /> : <Volume2 size={18} />}
                  </button>
                  <span className="font-mono text-[11px] text-[#AAAAAA]">
                    {formatTime(progressSec)} / {formatTime(totalDurationSec)}
                  </span>
                </div>

                <div className="flex items-center gap-3">
                  <button
                    onClick={() => setProgressSec(0)}
                    className="hover:text-white text-[#AAAAAA] transition-colors"
                    title="Restart"
                  >
                    <RotateCcw size={16} />
                  </button>
                  <button className="hover:text-white text-[#AAAAAA] transition-colors">
                    <Maximize2 size={18} />
                  </button>
                </div>
              </div>
            </div>
          </div>

          {/* Video Title */}
          <h2 className="text-xl font-bold text-[#F1F1F1] leading-snug">
            {video.title}
          </h2>

          {/* Channel Bar & Action Buttons */}
          <div className="flex flex-wrap items-center justify-between gap-4 py-2 border-b border-[#303030]">
            {/* Channel Info */}
            <div className="flex items-center gap-3">
              <div
                className="w-10 h-10 rounded-full flex items-center justify-center font-bold text-white text-sm shadow-md"
                style={{ backgroundColor: video.channelAvatarBg }}
              >
                {video.channelAvatarText}
              </div>
              <div className="flex flex-col">
                <div className="flex items-center gap-1.5">
                  <span className="text-sm font-bold text-white">{video.channel}</span>
                  {video.isVerified && (
                    <CheckCircle2 size={13} className="text-[#AAAAAA] fill-[#AAAAAA]" />
                  )}
                </div>
                <span className="text-xs text-[#AAAAAA]">{video.subscribers}</span>
              </div>

              <button
                onClick={() => setIsSubscribed(!isSubscribed)}
                className={`ml-4 px-4 py-2 rounded-full text-xs font-bold transition-all cursor-pointer ${
                  isSubscribed
                    ? 'bg-[#272727] text-white hover:bg-[#3F3F3F]'
                    : 'bg-white text-black hover:bg-[#F2F2F2]'
                }`}
              >
                {isSubscribed ? 'Subscribed' : 'Subscribe'}
              </button>
            </div>

            {/* Actions Bar (Like, Dislike, Share, Save) */}
            <div className="flex items-center gap-2">
              <div className="flex items-center bg-[#272727] rounded-full overflow-hidden">
                <button
                  onClick={toggleLike}
                  className={`flex items-center gap-2 px-3.5 py-2 hover:bg-[#3F3F3F] transition-colors text-xs font-semibold cursor-pointer ${
                    isLiked ? 'text-[#3EA6FF]' : 'text-white'
                  }`}
                >
                  <ThumbsUp size={16} className={isLiked ? 'fill-current' : ''} />
                  <span>{likesCount.toLocaleString()}</span>
                </button>
                <div className="h-5 w-[1px] bg-[#3F3F3F]" />
                <button
                  onClick={toggleDislike}
                  className={`px-3 py-2 hover:bg-[#3F3F3F] transition-colors text-xs cursor-pointer ${
                    isDisliked ? 'text-[#3EA6FF]' : 'text-white'
                  }`}
                >
                  <ThumbsDown size={16} className={isDisliked ? 'fill-current' : ''} />
                </button>
              </div>

              <button className="flex items-center gap-2 px-3.5 py-2 rounded-full bg-[#272727] hover:bg-[#3F3F3F] text-xs font-semibold text-white transition-colors cursor-pointer">
                <Share2 size={16} />
                <span>Share</span>
              </button>

              <button className="hidden sm:flex items-center gap-2 px-3.5 py-2 rounded-full bg-[#272727] hover:bg-[#3F3F3F] text-xs font-semibold text-white transition-colors cursor-pointer">
                <Download size={16} />
                <span>Download</span>
              </button>

              <button className="p-2 rounded-full bg-[#272727] hover:bg-[#3F3F3F] text-white transition-colors cursor-pointer">
                <MoreHorizontal size={18} />
              </button>
            </div>
          </div>

          {/* Description Box */}
          <div
            onClick={() => setIsDescExpanded(!isDescExpanded)}
            className="p-3.5 rounded-xl bg-[#272727]/80 hover:bg-[#272727] transition-colors cursor-pointer text-xs leading-relaxed space-y-2"
          >
            <div className="flex items-center gap-3 font-bold text-white">
              <span>{video.views}</span>
              <span>{video.uploadTime}</span>
              <span className="px-2 py-0.5 rounded bg-white/10 text-[10px] text-[#3EA6FF]">
                #{video.category}
              </span>
            </div>
            <p className={isDescExpanded ? 'text-zinc-300' : 'text-zinc-300 line-clamp-2'}>
              {video.description}
            </p>
            <span className="text-[11px] font-bold text-[#AAAAAA] hover:text-white">
              {isDescExpanded ? 'Show less' : '...more'}
            </span>
          </div>
        </div>

        {/* Right Sidebar: Up Next / Recommended List */}
        <div className="w-full lg:w-[380px] xl:w-[420px] shrink-0 flex flex-col gap-4">
          <div className="flex items-center justify-between pb-1 border-b border-[#303030]">
            <span className="text-sm font-bold text-white">Up Next</span>
            <span className="text-xs text-[#AAAAAA]">Autoplay ON</span>
          </div>

          <div className="flex flex-col gap-3">
            {relatedVideos.map((item) => (
              <div
                key={item.id}
                onClick={() => onSelectRelated(item)}
                className="flex gap-3 group cursor-pointer p-1.5 rounded-xl hover:bg-[#272727] transition-colors"
              >
                {/* 16:9 Mini Thumbnail */}
                <div
                  className="relative w-40 aspect-video rounded-lg overflow-hidden shrink-0 flex items-center justify-center shadow-md"
                  style={{ background: item.thumbnailGradient }}
                >
                  <span className="text-[9px] font-bold text-white/90 px-1.5 py-0.5 rounded bg-black/40 text-center truncate max-w-[90%]">
                    {item.thumbnailTag}
                  </span>
                  <span className="absolute bottom-1 right-1 px-1.5 py-0.5 rounded bg-black/80 text-[10px] font-semibold text-white">
                    {item.duration}
                  </span>
                </div>

                {/* Video Info */}
                <div className="flex-1 flex flex-col justify-between py-0.5">
                  <h4 className="text-xs font-bold text-white group-hover:text-[#3EA6FF] line-clamp-2 leading-snug transition-colors">
                    {item.title}
                  </h4>
                  <div className="flex flex-col text-[11px] text-[#AAAAAA]">
                    <span className="flex items-center gap-1 hover:text-white">
                      {item.channel}
                      {item.isVerified && <CheckCircle2 size={10} className="fill-[#AAAAAA]" />}
                    </span>
                    <span>
                      {item.views} • {item.uploadTime}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
