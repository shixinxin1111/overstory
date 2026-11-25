import type { ComponentType } from 'react';
import type { Presentation, Snapshot } from '../shared/types';
import { TodoView } from '../modules/todos/TodoView';
import { BookmarkView } from '../modules/bookmarks/BookmarkView';
import { NotesView } from '../modules/notes/NotesView';
import type { IconName } from '../shared/icons';

export type ModuleDefinition = {
  id: string; title: string; icon: IconName; order: number; dataVersion: number;
  View: ComponentType<{ presentation: Exclude<Presentation, 'orb'> }>;
  getOrbSummary?: (snapshot: Snapshot) => string | number | null;
};
export const modules: ModuleDefinition[] = [
  { id: 'todos', title: '待办', icon: 'todos', order: 0, dataVersion: 1, View: TodoView, getOrbSummary: s => s.modules.todos.data.items.filter(i => !i.completed).length },
  { id: 'bookmarks', title: '网页', icon: 'bookmarks', order: 1, dataVersion: 1, View: BookmarkView },
  { id: 'notes', title: '便签', icon: 'notes', order: 2, dataVersion: 1, View: NotesView },
];
export function orderedModules(order: string[]) {
  const byId = new Map(modules.map(m => [m.id, m]));
  return [...order.flatMap(id => byId.has(id) ? [byId.get(id)!] : []), ...modules.filter(m => !order.includes(m.id)).sort((a,b) => a.order - b.order)];
}
