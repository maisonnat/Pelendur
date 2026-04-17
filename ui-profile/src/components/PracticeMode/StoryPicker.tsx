import { FC } from 'react';

interface StarStory {
  id: string;
  title: string;
  situation: string;
  task: string;
  action: string;
  result: string;
  tags: string[];
}

interface StoryPickerProps {
  stories: StarStory[];
}

export const StoryPicker: FC<StoryPickerProps> = ({ stories }) => {
  if (stories.length === 0) {
    return null;
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-white/70">
          Relevant STAR Stories
        </h3>
        <span className="text-xs text-white/40">
          {stories.length} available
        </span>
      </div>
      <div className="space-y-2">
        {stories.map((story) => (
          <div key={story.id} className="bg-gray-800/20 border border-gray-700/30 rounded-lg p-3 hover:bg-gray-800/30 transition-colors">
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <h4 className="font-medium text-white mb-1">
                  {story.title}
                </h4>
                <p className="text-xs text-white/50 truncate">
                  {story.situation}
                </p>
              </div>
              <div className="flex items-center gap-2 text-xs">
                {story.tags.slice(0, 3).map((tag) => (
                  <span key={tag} className="px-2 py-0.5 bg-white/10 text-white/60 rounded text-[9px]">
                    #{tag}
                  </span>
                ))}
              </div>
            </div>
            <div className="mt-2 text-xs text-white/60 space-y-1">
              <div><strong>T:</strong> {story.task}</div>
              <div><strong>A:</strong> {story.action}</div>
              <div><strong>R:</strong> {story.result}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};