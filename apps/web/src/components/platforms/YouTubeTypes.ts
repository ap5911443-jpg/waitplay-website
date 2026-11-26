export interface YouTubeVideo {
  id: string;
  title: string;
  channel: {
    name: string;
    avatar: string;
    subscribers: string;
    verified: boolean;
    isSubscribed?: boolean;
  };
  category: string;
  views: string;
  viewsCount: number;
  uploadedAt: string;
  duration: string;
  thumbnailUrl: string;
  badge?: string;
  likes: string;
  likesCount: number;
  description: string;
  commentsCount: number;
  comments: {
    author: string;
    avatar: string;
    text: string;
    timeAgo: string;
    likes: number;
  }[];
}

export const YOUTUBE_CATEGORIES = [
  "All",
  "AI",
  "Startup company",
  "Web series",
  "Podcasts",
  "Sitcoms",
  "Music",
  "Mixes",
  "Romantic comedies",
  "Thrillers",
  "Masala films",
  "Theatre",
  "Live",
  "Stocks",
  "Information",
];

export const INITIAL_YOUTUBE_VIDEOS: YouTubeVideo[] = [
  {
    id: "yt-1",
    title: "How Open Source Founders Make Millions: From GitHub Star to $100M Valuation",
    channel: {
      name: "DevFounders",
      avatar: "https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=150&auto=format&fit=crop&q=80",
      subscribers: "840K subscribers",
      verified: true,
      isSubscribed: false,
    },
    category: "Startup company",
    views: "482K views",
    viewsCount: 482000,
    uploadedAt: "3 days ago",
    duration: "14:20",
    thumbnailUrl: "https://images.unsplash.com/photo-1551288049-bebda4e38f71?w=800&auto=format&fit=crop&q=80",
    badge: "SAAS BREAKDOWN",
    likes: "32K",
    likesCount: 32000,
    description: "An in-depth breakdown of how developer-first open source companies monetize enterprise features, cloud hosting, and self-hosted support contracts to scale from zero to unicorn.",
    commentsCount: 640,
    comments: [
      {
        author: "sarah_codes",
        avatar: "https://images.unsplash.com/photo-1494790108377-be9c29b29330?w=150&auto=format&fit=crop&q=80",
        text: "The breakdown of commercial open source licenses vs SaaS cloud wrappers was so clear!",
        timeAgo: "2 days ago",
        likes: 142,
      },
      {
        author: "alex_founder",
        avatar: "https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=150&auto=format&fit=crop&q=80",
        text: "Best tech business analysis on YouTube right now.",
        timeAgo: "1 day ago",
        likes: 88,
      },
    ],
  },
  {
    id: "yt-2",
    title: "AI Marketing: Big Money, Algorithmic Virality, and Autonomous Growth Engines",
    channel: {
      name: "Modern Growth",
      avatar: "https://images.unsplash.com/photo-1500648767791-00dcc994a43e?w=150&auto=format&fit=crop&q=80",
      subscribers: "1.2M subscribers",
      verified: true,
      isSubscribed: true,
    },
    category: "AI",
    views: "1.1M views",
    viewsCount: 1100000,
    uploadedAt: "1 week ago",
    duration: "18:45",
    thumbnailUrl: "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=800&auto=format&fit=crop&q=80",
    badge: "VIRAL CASE STUDY",
    likes: "85K",
    likesCount: 85000,
    description: "How modern AI companies generate millions of organic impressions using programmatic video engines, AI avatar pipelines, and autonomous marketing agents.",
    commentsCount: 1250,
    comments: [
      {
        author: "marketing_pro",
        avatar: "https://images.unsplash.com/photo-1522075469751-3a6694fb2f61?w=150&auto=format&fit=crop&q=80",
        text: "Programmatic video rendering with Remotion and AI scriptwriters is the real secret sauce.",
        timeAgo: "5 days ago",
        likes: 310,
      },
    ],
  },
  {
    id: "yt-3",
    title: "Dhindora | EP 02 | The Unstoppable Chain Reaction",
    channel: {
      name: "BB Ki Vines Studios",
      avatar: "https://images.unsplash.com/photo-1472099645785-5658abf4ff4e?w=150&auto=format&fit=crop&q=80",
      subscribers: "26.4M subscribers",
      verified: true,
      isSubscribed: true,
    },
    category: "Web series",
    views: "34M views",
    viewsCount: 34000000,
    uploadedAt: "2 weeks ago",
    duration: "22:15",
    thumbnailUrl: "https://images.unsplash.com/photo-1517604931442-7e0c8ed2963c?w=800&auto=format&fit=crop&q=80",
    badge: "EPISODE 02",
    likes: "2.4M",
    likesCount: 2400000,
    description: "The thrilling continuation of the lottery ticket saga. Comedy, drama, and unexpected twists as families and neighbors collide in this iconic web series.",
    commentsCount: 18400,
    comments: [
      {
        author: "rahul_vines",
        avatar: "https://images.unsplash.com/photo-1519085360753-af0119f7cbe7?w=150&auto=format&fit=crop&q=80",
        text: "The character transitions and background score in this episode are world-class!",
        timeAgo: "1 week ago",
        likes: 2400,
      },
    ],
  },
  {
    id: "yt-4",
    title: "Mastercard's Invisible Empire & The Architecture of Global Payment Rails",
    channel: {
      name: "Fintech Decoded",
      avatar: "https://images.unsplash.com/photo-1519085360753-af0119f7cbe7?w=150&auto=format&fit=crop&q=80",
      subscribers: "650K subscribers",
      verified: true,
      isSubscribed: false,
    },
    category: "Stocks",
    views: "890K views",
    viewsCount: 890000,
    uploadedAt: "5 days ago",
    duration: "16:10",
    thumbnailUrl: "https://images.unsplash.com/photo-1559526324-4b87b5e36e44?w=800&auto=format&fit=crop&q=80",
    badge: "FINTECH MONOPOLY",
    likes: "61K",
    likesCount: 61000,
    description: "Every credit card swipe triggers an elaborate settlement dance involving merchant acquiring banks, card networks, and issuing banks. Here is the code and infrastructure that runs it.",
    commentsCount: 920,
    comments: [
      {
        author: "fintech_nerd",
        avatar: "https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=150&auto=format&fit=crop&q=80",
        text: "Interchange fees and settlement clearing explained better than any textbook.",
        timeAgo: "3 days ago",
        likes: 215,
      },
    ],
  },
  {
    id: "yt-5",
    title: "9 Minutes of Non-stop Laughter: Standup Comedy Specials & Epic Crowd Work",
    channel: {
      name: "Comedy Hub Live",
      avatar: "https://images.unsplash.com/photo-1522075469751-3a6694fb2f61?w=150&auto=format&fit=crop&q=80",
      subscribers: "1.8M subscribers",
      verified: true,
      isSubscribed: false,
    },
    category: "Sitcoms",
    views: "2.8M views",
    viewsCount: 2800000,
    uploadedAt: "4 days ago",
    duration: "09:00",
    thumbnailUrl: "https://images.unsplash.com/photo-1514306191717-452ec28c7814?w=800&auto=format&fit=crop&q=80",
    badge: "BEST OF 2026",
    likes: "190K",
    likesCount: 190000,
    description: "The most hilarious standup crowd interactions, quick comebacks, and stage moments gathered into 9 minutes of pure joy while you wait.",
    commentsCount: 3100,
    comments: [
      {
        author: "laugh_tracker",
        avatar: "https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=150&auto=format&fit=crop&q=80",
        text: "I literally cried laughing at the pilot bit at 4:30 😂",
        timeAgo: "2 days ago",
        likes: 450,
      },
    ],
  },
  {
    id: "yt-6",
    title: "Vibe Play Architecture Deep Dive: Turning AI Agent Latency Into Flow State",
    channel: {
      name: "Vibe Play Official",
      avatar: "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=150&auto=format&fit=crop&q=80",
      subscribers: "150K subscribers",
      verified: true,
      isSubscribed: true,
    },
    category: "AI",
    views: "320K views",
    viewsCount: 320000,
    uploadedAt: "1 day ago",
    duration: "11:50",
    thumbnailUrl: "https://images.unsplash.com/photo-1526374965328-7f61d4dc18c5?w=800&auto=format&fit=crop&q=80",
    badge: "SYSTEM DESIGN",
    likes: "42K",
    likesCount: 42000,
    description: "How Vibe Play bridges local terminal agents (Claude Code, Codex, Opencode) over loopback WebSockets to spawn instant micro-interactions and keep developers hyper-focused.",
    commentsCount: 520,
    comments: [
      {
        author: "dev_dan",
        avatar: "https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=150&auto=format&fit=crop&q=80",
        text: "Zero context switching overhead while tests and builds execute. Essential tool.",
        timeAgo: "18h ago",
        likes: 85,
      },
    ],
  },
];
