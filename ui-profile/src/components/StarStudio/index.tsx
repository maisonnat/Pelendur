import { useState, useEffect, useCallback } from "react";
import {
  getStarStories,
  createStarStory,
  updateStarStory,
  deleteStarStory,
  type StarStoryRecord,
} from "../../lib/ipc";
import { type StoryFormData, EMPTY_STORY } from "./types";
import StoryList from "./StoryList";
import StoryEditor from "./StoryEditor";
import StoryPreview from "./StoryPreview";
import CoachingPanel from "./CoachingPanel";
import PracticeMode from "./PracticeMode";

type Panel = "list" | "editor" | "preview";

export default function StarStudio() {
  const [stories, setStories] = useState<StarStoryRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [panel, setPanel] = useState<Panel>("list");
  const [editData, setEditData] = useState<StoryFormData>(EMPTY_STORY);
  const [isNew, setIsNew] = useState(false);
  const [showCoach, setShowCoach] = useState(false);
  const [showPractice, setShowPractice] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");

  const loadStories = useCallback(async () => {
    setLoading(true);
    try {
      const result = await getStarStories();
      setStories(result);
    } catch (e) {
      console.error("Failed to load stories:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStories();
  }, [loadStories]);

  const selectedStory = stories.find((s) => s.id === selectedId) ?? null;

  const handleSelect = (id: string) => {
    setSelectedId(id);
    setPanel("preview");
    setShowCoach(false);
  };

  const handleNew = () => {
    setSelectedId(null);
    setEditData({ ...EMPTY_STORY });
    setIsNew(true);
    setPanel("editor");
    setShowCoach(false);
  };

  const handleEdit = () => {
    if (!selectedStory) return;
    const tags = (() => {
      if (!selectedStory.tags) return [];
      try {
        return JSON.parse(selectedStory.tags);
      } catch {
        return [];
      }
    })();

    setEditData({
      id: selectedStory.id,
      title: selectedStory.title ?? "",
      situation: selectedStory.situation,
      task: selectedStory.task,
      action: selectedStory.action,
      result: selectedStory.result,
      tags,
      difficulty: selectedStory.difficulty ?? "",
      stakes: selectedStory.stakes ?? "",
    });
    setIsNew(false);
    setPanel("editor");
  };

  const handleSave = async () => {
    const tagsJson = editData.tags.length > 0 ? JSON.stringify(editData.tags) : null;
    const payload = {
      id: editData.id,
      title: editData.title || null,
      situation: editData.situation,
      task: editData.task,
      action: editData.action,
      result: editData.result,
      tags: tagsJson,
      difficulty: editData.difficulty || null,
      stakes: editData.stakes || null,
    };

    try {
      if (isNew) {
        const created = await createStarStory(payload);
        setSelectedId(created.id);
      } else {
        await updateStarStory(payload);
      }
      await loadStories();
      setPanel("preview");
    } catch (e) {
      console.error("Failed to save story:", e);
    }
  };

  const handleDelete = async () => {
    if (!selectedId) return;
    try {
      await deleteStarStory(selectedId);
      setSelectedId(null);
      setPanel("list");
      await loadStories();
    } catch (e) {
      console.error("Failed to delete story:", e);
    }
  };

  const handleCancel = () => {
    if (selectedId) {
      setPanel("preview");
    } else {
      setPanel("list");
    }
  };

  const handleOpenCoach = () => {
    setShowCoach(true);
  };

  return (
    <div className="h-full flex flex-col -m-6">
      <div className="flex items-center justify-between px-6 py-4 border-b border-white/[0.04] bg-[rgba(15,15,15,0.6)]">
        <div className="flex items-center gap-3">
          <h2 className="text-xl font-semibold text-[#ffd700]">STAR Story Studio</h2>
          <span className="text-xs text-white/15">{stories.length} stories</span>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setShowPractice(true)}
            className="px-3 py-1.5 text-xs rounded-lg bg-[rgba(255,215,0,0.08)] text-[#ffd700]/60 border border-[rgba(255,215,0,0.1)] hover:bg-[rgba(255,215,0,0.14)] transition-all"
          >
            ✦ Practice
          </button>
          <button
            type="button"
            onClick={handleNew}
            className="px-3 py-1.5 text-xs rounded-lg bg-[rgba(255,215,0,0.1)] text-[#ffd700] border border-[rgba(255,215,0,0.15)] hover:bg-[rgba(255,215,0,0.18)] transition-all"
          >
            + New Story
          </button>
        </div>
      </div>

      <div className="flex-1 flex min-h-0">
        <div className={`w-72 shrink-0 border-r border-white/[0.04] bg-[rgba(18,18,18,0.5)] overflow-hidden ${showCoach ? "hidden lg:block" : ""}`}>
          <StoryList
            stories={stories}
            selectedId={selectedId}
            onSelect={handleSelect}
            onNew={handleNew}
            searchQuery={searchQuery}
            onSearchChange={setSearchQuery}
          />
        </div>

        <div className="flex-1 min-w-0 overflow-y-auto">
          {loading && stories.length === 0 ? (
            <div className="flex items-center justify-center h-full">
              <p className="text-xs text-white/20">Loading stories...</p>
            </div>
          ) : panel === "editor" ? (
            <div className="max-w-2xl mx-auto p-6">
              <StoryEditor
                story={editData}
                onChange={setEditData}
                onSave={handleSave}
                onCancel={handleCancel}
                isNew={isNew}
              />
            </div>
          ) : panel === "preview" && selectedStory ? (
            <div className="max-w-2xl mx-auto p-6">
              <StoryPreview
                story={selectedStory}
                onEdit={handleEdit}
                onDelete={handleDelete}
                onClose={() => {
                  setSelectedId(null);
                  setPanel("list");
                }}
                onCoach={handleOpenCoach}
              />
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="text-4xl text-white/[0.04]">★</div>
              <p className="text-sm text-white/15">Select a story or create a new one</p>
              <button
                type="button"
                onClick={handleNew}
                className="px-4 py-2 text-xs rounded-lg bg-[rgba(255,215,0,0.08)] text-[#ffd700]/60 border border-[rgba(255,215,0,0.12)] hover:bg-[rgba(255,215,0,0.14)] transition-all"
              >
                Create Your First Story
              </button>
            </div>
          )}
        </div>

        {showCoach && (
          <div className="w-80 shrink-0 border-l border-white/[0.04] bg-[rgba(18,18,18,0.5)]">
            <CoachingPanel
              storyId={selectedId}
              storyTitle={selectedStory?.title ?? ""}
              onClose={() => setShowCoach(false)}
            />
          </div>
        )}
      </div>

      {showPractice && (
        <PracticeMode
          stories={stories}
          onClose={() => setShowPractice(false)}
        />
      )}
    </div>
  );
}
