// Development-only adapter. Vite removes this import from production builds.
import { emptySnapshot } from './types';
import type { Item, ModuleData, Snapshot, Preferences, Presentation } from './types';
let snapshot: Snapshot = emptySnapshot();
snapshot.windowState.mode = new URLSearchParams(location.search).get('mode') as Presentation || 'main';
const time = new Date().toISOString();
snapshot.modules.todos.data.items = ['整理今天的想法', '留半小时读一本书', '把灵感放进便签'].map((title, n) => ({ id: `preview-todo-${n}`, title, completed: false, sortIndex: n, createdAt: time, updatedAt: time }));
snapshot.modules.bookmarks.data.items = [{ id: 'preview-web', title: '设计灵感', url: 'https://example.com/', groupId: 'ungrouped', sortIndex: 0, createdAt: time, updatedAt: time }];
snapshot.modules.notes.data.items = [{ id: 'preview-note', title: '从一个想法开始', content: '记录那些一闪而过的好点子。\n\n给下一次灵感，留一个位置。', groupId: 'ungrouped', sortIndex: 0, createdAt: time, updatedAt: time }];
snapshot.session.selectedNoteId = 'preview-note';
for (const namespace of Object.values(snapshot.modules)) {
  namespace.data.items.forEach((item, n) => { item.favorited = true; item.favoriteIndex = n; });
}
const emit = (name: string, detail: unknown) => window.dispatchEvent(new CustomEvent(name, { detail }));
function normalize(d: ModuleData) {
  const counts = new Map<string, number>();
  d.items.forEach(i => { const group = i.groupId ?? ''; i.sortIndex = counts.get(group) ?? 0; counts.set(group, i.sortIndex + 1); });
  d.groups.forEach((g, n) => { g.sortIndex = n; });
  const favorites = d.items.filter(i => i.favorited).sort((a, b) => (a.favoriteIndex ?? Infinity) - (b.favoriteIndex ?? Infinity));
  d.items.forEach(i => { delete i.favoriteIndex; });
  favorites.forEach((i, n) => { i.favoriteIndex = n; });
}
export async function previewInvoke<T>(command: string, args: Record<string, unknown>): Promise<T> {
  if (command === 'cli_status') return {
    installed: false, version: null, bundledVersion: '1.1.1', path: '/usr/local/bin/overstory',
    canInstall: false, canUpgrade: false, canUninstall: false, message: '请在桌面应用中管理 CLI。'
  } as T;
  if (command === 'manage_cli') throw new Error('请在桌面应用中管理 CLI');
  if (command === 'skill_status') return {
    installed: false, version: null, bundledVersion: '1.1.1', path: '~/.agents/skills/overstory',
    canInstall: false, canUpgrade: false, canUninstall: false, message: '请在桌面应用中管理 SKILL。'
  } as T;
  if (command === 'manage_skill') throw new Error('请在桌面应用中管理 SKILL');
  if (command === 'get_snapshot') return structuredClone(snapshot) as T;
  if (command === 'renderer_ready') return undefined as T;
  if (command === 'request_presentation') {
    emit('app:prepare-transition', { token: String(args.presentation), target: args.presentation }); return undefined as T;
  }
  if (command === 'acknowledge_transition') {
    snapshot.windowState.mode = args.token as Presentation;
    snapshot.session = args.session as Snapshot['session']; snapshot.revision++;
    emit('app:snapshot-changed', structuredClone(snapshot)); return undefined as T;
  }
  if (command === 'orb_menu') { emit('app:prepare-transition', { token: 'main', target: 'main' }); return undefined as T; }
  if (['open_bookmark', 'open_external', 'export_backup', 'import_backup'].includes(command)) throw new Error('开发预览使用隔离数据，请在桌面应用中验证此操作');
  if (command === 'update_preferences') snapshot.preferences = args.preferences as Preferences;
  else if (command === 'execute_module_action') {
    const module = String(args.moduleId); const d = snapshot.modules[module].data;
    const a = args.action as Record<string, unknown>;
    const id = String(a.id ?? a.itemId ?? a.groupId ?? '');
    if (a.type === 'create') {
      if (!String(a.title).trim()) throw new Error('标题不能为空');
      const item: Item = { id: crypto.randomUUID(), title: String(a.title), sortIndex: 0, createdAt: time, updatedAt: time };
      item.favorited = a.favorited === true;
      if (item.favorited) item.favoriteIndex = d.items.filter(i => i.favorited).length;
      if (module === 'todos') item.completed = false;
      else item.groupId = String(a.groupId ?? 'ungrouped');
      if (module === 'notes') item.content = String(a.content ?? '');
      if (module === 'bookmarks') {
        item.url = String(a.url).includes('://') ? String(a.url) : `https://${a.url}`;
        if (a.icon) item.icon = new URL(String(a.icon).includes('://') ? String(a.icon) : `https://${a.icon}`).href;
        if (a.description !== undefined) item.description = String(a.description);
      }
      d.items.push(item);
    } else if (a.type === 'update') {
      const item = d.items.find(i => i.id === id)!;
      if (a.expectedItemVersion !== undefined && a.expectedItemVersion !== (item.version ?? 0)) throw { code: 'ITEM_CONFLICT', message: '内容已更新，草稿已保留。' };
      for (const field of ['title','content','url','description','completed','groupId']) if (a[field] !== undefined) Object.assign(item, { [field]: a[field] });
      if (a.icon !== undefined) {
        if (a.icon) item.icon = new URL(String(a.icon).includes('://') ? String(a.icon) : `https://${a.icon}`).href;
        else delete item.icon;
      }
      item.version = (item.version ?? 0) + 1;
    } else if (a.type === 'set_favorite') {
      const item = d.items.find(i => i.id === id);
      if (!item) throw new Error('项目不存在');
      if (item.favorited !== a.favorited) {
        item.favoriteIndex = a.favorited ? d.items.filter(i => i.favorited).length : undefined;
        item.favorited = a.favorited === true;
      }
    } else if (a.type === 'move_favorite') {
      if (a.expectedRevision !== snapshot.revision) throw { code: 'STALE_REVISION', message: '数据已更新，请重新拖动' };
      const favorites = d.items.filter(i => i.favorited).sort((a, b) => (a.favoriteIndex ?? 0) - (b.favoriteIndex ?? 0));
      const at = favorites.findIndex(i => i.id === id);
      if (at < 0 || a.beforeItemId && !favorites.some(i => i.id === a.beforeItemId)) throw new Error('收藏排序目标不存在');
      if (a.beforeItemId !== id) {
        const [item] = favorites.splice(at, 1);
        const target = a.beforeItemId ? favorites.findIndex(i => i.id === a.beforeItemId) : favorites.length;
        favorites.splice(target, 0, item);
        favorites.forEach((i, n) => { i.favoriteIndex = n; });
      }
    } else if (a.type === 'delete') d.items = d.items.filter(i => i.id !== id);
    else if (a.type === 'create_group') d.groups.push({ id: crypto.randomUUID(), name: String(a.name), sortIndex: d.groups.length, createdAt: time });
    else if (a.type === 'rename_group') d.groups.find(g => g.id === id)!.name = String(a.name);
    else if (a.type === 'delete_group') { d.items.filter(i => i.groupId === id).forEach(i => { i.groupId = 'ungrouped'; }); d.groups = d.groups.filter(g => g.id !== id); }
    else if (a.type === 'move_item') {
      const item = d.items.find(i => i.id === id)!; d.items = d.items.filter(i => i.id !== id);
      if (a.toGroupId) item.groupId = String(a.toGroupId);
      const at = a.beforeItemId ? d.items.findIndex(i => i.id === a.beforeItemId) : d.items.length;
      d.items.splice(at < 0 ? d.items.length : at, 0, item);
    } else if (a.type === 'move_group') {
      const group = d.groups.find(g => g.id === id)!; d.groups = d.groups.filter(g => g.id !== id);
      const at = a.beforeGroupId ? d.groups.findIndex(g => g.id === a.beforeGroupId) : d.groups.length;
      d.groups.splice(at < 0 ? d.groups.length : at, 0, group);
    }
    normalize(d);
  } else throw new Error(`开发预览不支持：${command}`);
  snapshot.revision++; const result = structuredClone(snapshot); emit('app:snapshot-changed', result); return result as T;
}
