export const invoke = async (cmd: string, args?: Record<string, unknown>): Promise<unknown> => {
  console.warn(`[Tauri Mock] invoke called: ${cmd}`);
  return null;
};
