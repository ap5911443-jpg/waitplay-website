import React, { useState, useEffect } from "react";
import { Download, Menu, X, Github } from "lucide-react";
import { PrimaryButton } from "../ui/PrimaryButton";

export const SiteHeader: React.FC = () => {
  const [scrolled, setScrolled] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      setScrolled(window.scrollY > 20);
    };
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  const navLinks = [
    { label: "Overview", href: "#overview" },
    { label: "Speed", href: "#speed" },
    { label: "How It Works", href: "#how-it-works" },
    { label: "Personalize", href: "#personalize" },
    { label: "Security", href: "#security" },
    { label: "FAQ", href: "#faq" },
  ];

  return (
    <header className="fixed top-0 left-0 right-0 z-50 px-4 sm:px-6 lg:px-8 py-3.5 transition-all duration-200">
      <div
        className={`max-w-[1240px] mx-auto rounded-[20px] transition-all duration-300 border ${
          scrolled
            ? "bg-[#FFFFEB]/90 backdrop-blur-md border-[rgba(26,26,26,0.14)] shadow-[0_4px_20px_rgba(26,26,15,0.06)] py-2.5 px-4 sm:px-6"
            : "bg-[#FFFFEB]/70 backdrop-blur-sm border-[rgba(26,26,26,0.08)] py-3 px-4 sm:px-6"
        }`}
      >
        <div className="flex items-center justify-between">
          {/* Brand Logo & Wordmark */}
          <a href="#" className="flex items-center gap-2.5 group">
            <img src="/logo.png" alt="" width={32} height={32} className="w-8 h-8 transition-transform duration-200 group-hover:scale-105" />
            <div className="flex items-baseline gap-1">
              <span className="font-serif text-[21px] font-normal tracking-tight text-[#1A1A1A]">
                Vibe<span className="font-sans font-semibold text-[19px] text-[#034F46]"> Play</span>
              </span>
              <span className="w-1.5 h-1.5 rounded-full bg-[#FFA946] animate-pulse ml-0.5" />
            </div>
          </a>

          {/* Desktop Navigation Links */}
          <nav className="hidden md:flex items-center gap-7">
            {navLinks.map((link) => (
              <a
                key={link.label}
                href={link.href}
                className="font-sans text-[14.5px] font-medium text-[#1A1A1A]/80 hover:text-[#1A1A1A] transition-colors relative py-1 after:absolute after:bottom-0 after:left-0 after:w-0 after:h-[1.5px] after:bg-[#034F46] hover:after:w-full after:transition-all after:duration-200"
              >
                {link.label}
              </a>
            ))}
          </nav>

          {/* Right Action Buttons */}
          <div className="hidden sm:flex items-center gap-3">
            <a
              href="https://github.com/ap5911443-jpg/waitplay-website"
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[13px] font-medium text-[#666666] hover:text-[#1A1A1A] hover:bg-[#E4E4D0]/50 transition-colors"
            >
              <Github className="w-4 h-4" />
              <span>Star</span>
              <span className="text-[11px] px-1.5 py-0.5 rounded bg-[#E4E4D0] font-semibold text-[#1A1A1A]">
                1.4k
              </span>
            </a>

            <PrimaryButton
              size="sm"
              href="#download"
              icon={<Download className="w-3.5 h-3.5" />}
              className="text-[13.5px] py-2 px-4"
            >
              Download Free
            </PrimaryButton>
          </div>

          {/* Mobile Menu Toggle */}
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="md:hidden p-2 rounded-lg text-[#1A1A1A] hover:bg-[#E4E4D0]/60 transition-colors focus:outline-none"
            aria-label="Toggle menu"
          >
            {mobileMenuOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
          </button>
        </div>

        {/* Mobile Dropdown Menu */}
        {mobileMenuOpen && (
          <div className="md:hidden mt-3 pt-3 border-t border-[rgba(26,26,26,0.1)] flex flex-col gap-3 pb-2 animate-fadeIn">
            {navLinks.map((link) => (
              <a
                key={link.label}
                href={link.href}
                onClick={() => setMobileMenuOpen(false)}
                className="font-sans text-[15px] font-medium text-[#1A1A1A] py-1.5 px-2 rounded-md hover:bg-[#E4E4D0]/50 transition-colors"
              >
                {link.label}
              </a>
            ))}
            <div className="pt-2 border-t border-[rgba(26,26,26,0.08)] flex flex-col gap-2">
              <PrimaryButton
                size="md"
                href="#download"
                icon={<Download className="w-4 h-4" />}
                onClick={() => setMobileMenuOpen(false)}
                className="w-full"
              >
                Download Free
              </PrimaryButton>
            </div>
          </div>
        )}
      </div>
    </header>
  );
};
