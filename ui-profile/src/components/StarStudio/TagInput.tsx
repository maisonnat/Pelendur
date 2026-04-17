import { useState, useRef, useEffect, useCallback } from "react";
import { SUGGESTED_TAGS } from "./types";

interface TagInputProps {
  tags: string[];
  onChange: (tags: string[]) => void;
}

export default function TagInput({ tags, onChange }: TagInputProps) {
  const [input, setInput] = useState("");
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [highlighted, setHighlighted] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const filtered = SUGGESTED_TAGS.filter(
    (t) => t.toLowerCase().includes(input.toLowerCase()) && !tags.includes(t)
  );

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setShowSuggestions(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const addTag = useCallback(
    (tag: string) => {
      const normalized = tag.trim().toLowerCase();
      if (normalized && !tags.includes(normalized)) {
        onChange([...tags, normalized]);
      }
      setInput("");
      setShowSuggestions(false);
      setHighlighted(-1);
      inputRef.current?.focus();
    },
    [tags, onChange]
  );

  const removeTag = useCallback(
    (tag: string) => {
      onChange(tags.filter((t) => t !== tag));
    },
    [tags, onChange]
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Backspace" && input === "" && tags.length > 0) {
      removeTag(tags[tags.length - 1]);
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      if (highlighted >= 0 && highlighted < filtered.length) {
        addTag(filtered[highlighted]);
      } else if (input.trim()) {
        addTag(input);
      }
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlighted((h) => Math.min(h + 1, filtered.length - 1));
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlighted((h) => Math.max(h - 1, -1));
      return;
    }
    if (e.key === "Escape") {
      setShowSuggestions(false);
      setHighlighted(-1);
    }
  };

  return (
    <div ref={containerRef} className="relative">
      <div
        onClick={() => {
          inputRef.current?.focus();
          setShowSuggestions(true);
        }}
        className="flex flex-wrap gap-1.5 min-h-[38px] bg-white/[0.03] border border-white/[0.08] rounded-lg px-2.5 py-1.5 cursor-text focus-within:border-[#ffd700]/30 focus-within:bg-white/[0.05] transition-colors"
      >
        {tags.map((tag) => (
          <span
            key={tag}
            className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] font-medium bg-[rgba(255,215,0,0.08)] text-[#ffd700]/80 border border-[rgba(255,215,0,0.15)]"
          >
            {tag}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                removeTag(tag);
              }}
              className="text-white/30 hover:text-white/60 transition-colors leading-none text-xs"
            >
              ×
            </button>
          </span>
        ))}
        <input
          ref={inputRef}
          type="text"
          value={input}
          onChange={(e) => {
            setInput(e.target.value);
            setShowSuggestions(true);
            setHighlighted(-1);
          }}
          onFocus={() => setShowSuggestions(true)}
          onKeyDown={handleKeyDown}
          placeholder={tags.length === 0 ? "Add tags (press Enter)..." : ""}
          className="flex-1 min-w-[80px] bg-transparent text-xs text-white/70 placeholder-white/20 outline-none py-0.5"
        />
      </div>

      {showSuggestions && filtered.length > 0 && (
        <div className="absolute z-20 top-full left-0 right-0 mt-1 bg-[rgba(20,20,20,0.98)] border border-white/[0.08] rounded-lg shadow-2xl max-h-40 overflow-y-auto">
          {filtered.slice(0, 12).map((tag, i) => (
            <button
              key={tag}
              type="button"
              onClick={() => addTag(tag)}
              className={`w-full text-left px-3 py-1.5 text-xs transition-colors ${
                i === highlighted
                  ? "bg-[rgba(255,215,0,0.1)] text-[#ffd700]"
                  : "text-white/50 hover:text-white/70 hover:bg-white/[0.03]"
              }`}
            >
              {tag}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
