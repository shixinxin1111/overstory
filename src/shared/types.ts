export type Presentation = 'orb' | 'panel' | 'main';
export type ModuleId = 'todos' | 'bookmarks' | 'notes';
export type Item = {
  id: string; title: string; sortIndex: number; createdAt: string; updatedAt: string; version?: number;
  completed?: boolean; url?: string; icon?: string; description?: string; content?: string; groupId?: string;
  favorited?: boolean; favoriteIndex?: number;
};
export type Group = { id: string; name: string; sortIndex: number; createdAt: string };
export type ModuleData = { items: Item[]; groups: Group[] };
export type Preferences = { showTodoCount: boolean; moduleOrder: string[] };
export type Session = {
  activeModuleId: ModuleId; selectedNoteId: string | null;
  selectedGroupIds: Record<'bookmarks' | 'notes', string>;
  todoFilter: 'all' | 'active' | 'completed';
  search: Partial<Record<ModuleId, string>>;
  collapsed: Record<string, boolean>; scroll: Record<string, number>;
  todoInput?: string; todoAdding?: boolean; noteEditor?: boolean;
  bookmarkForm?: { id: string | null; title: string; url: string; icon?: string; description?: string; groupId: string; expectedItemVersion?: number } | null;
};
export type Snapshot = {
  schemaVersion: number; revision: number;
  modules: Record<string, { version: number; data: ModuleData }>;
  preferences: Preferences;
  windowState: { mode: Presentation };
  session: Session; recoveryNotice?: string | null;
};
export type AppError = { code: string; message: string };
export type ManagedToolStatus = {
  installed: boolean; version: string | null; bundledVersion: string; path: string;
  canInstall: boolean; canUpgrade: boolean; canUninstall: boolean; message: string | null;
};
export type CliStatus = ManagedToolStatus;
export type SkillStatus = ManagedToolStatus;
export const defaultSession = (): Session => ({
  activeModuleId: 'todos', selectedNoteId: null,
  selectedGroupIds: { bookmarks: 'all', notes: 'all' }, todoFilter: 'active',
  search: {}, collapsed: {}, scroll: {},
});
export const emptySnapshot = (): Snapshot => ({
  schemaVersion: 1, revision: 0,
  modules: Object.fromEntries(['todos', 'bookmarks', 'notes'].map(id => [id, {
    version: 1, data: { items: [], groups: id === 'todos' ? [] : [{ id: 'ungrouped', name: '未分组', sortIndex: 0, createdAt: new Date().toISOString() }] },
  }])),
  preferences: { showTodoCount: true, moduleOrder: ['todos', 'bookmarks', 'notes'] },
  windowState: { mode: 'orb' }, session: defaultSession(),
});
