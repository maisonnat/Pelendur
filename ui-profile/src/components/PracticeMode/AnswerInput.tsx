import { useState, useEffect, useRef, FC, ChangeEvent } from 'react';

interface AnswerInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export const AnswerInput: FC<AnswerInputProps> = ({ 
  value, 
  onChange, 
  placeholder = "Type your answer..." 
}) => {
  const [localValue, setLocalValue] = useState(value);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [isFocused, setIsFocused] = useState(false);
  const [charCount, setCharCount] = useState(0);

  // Sync local value with prop value
  useEffect(() => {
    setLocalValue(value);
  }, [value]);

  // Update char count
  useEffect(() => {
    setCharCount(localValue.length);
  }, [localValue]);

  const handleChange = (e: ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setLocalValue(newValue);
    onChange(newValue);
  };

  const handleFocus = () => setIsFocused(true);
  const handleBlur = () => setIsFocused(false);

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      textareaRef.current.style.height = `${textareaRef.current.scrollHeight}px`;
    }
  }, [localValue]);

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-white/40">
          {charCount} characters
        </span>
        {!isFocused && charCount === 0 && (
          <span className="text-xs text-white/20">
            STAR method recommended
          </span>
        )}
      </div>
      <textarea
        ref={textareaRef}
        value={localValue}
        onChange={handleChange}
        onFocus={handleFocus}
        onBlur={handleBlur}
        placeholder={placeholder}
        rows={4}
        className="w-full p-3 bg-gray-800/50 border border-gray-700 rounded-lg text-sm text-white placeholder-white/20 resize-none focus:outline-none focus:border-yellow-500/50"
        style={{ minHeight: '80px' }}
      />
    </div>
  );
};