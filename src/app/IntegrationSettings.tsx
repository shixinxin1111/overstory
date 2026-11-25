import { useEffect, useState } from 'react';
import { bridge } from '../shared/bridge';
import { Button, Confirm, IconButton, run, toast } from '../shared/components/Controls';
import type { ManagedToolStatus } from '../shared/types';

const tools = {
  cli: {
    title: '命令行工具', name: 'CLI', path: '/usr/local/bin/overstory',
    description: '使用 overstory 独立管理待办、网页和便签。App 关闭时也能使用，数据与 App 实时同步。',
    hint: '安装与卸载需要 macOS 管理员授权；卸载保留全部内容。',
    uninstall: '将移除 overstory 命令。App、待办、网页、便签和备份均会保留。',
    status: bridge.cliStatus, manage: bridge.manageCli,
  },
  skill: {
    title: '配套 SKILL', name: 'SKILL', path: '~/.agents/skills/overstory',
    description: '让豆包工作等客户端通过本机 CLI 管理 Overstory。技能安装到当前用户的共享目录，可在客户端技能列表中选择。',
    hint: '请同时安装命令行工具。在豆包工作中新建“本地电脑”任务；技能每次读取 CLI 的最新说明。',
    uninstall: '将移除当前用户的 Overstory 技能入口。其他技能、CLI、App 和全部内容均会保留。',
    status: bridge.skillStatus, manage: bridge.manageSkill,
  },
};

const errorMessage = (error: unknown) => error instanceof Error ? error.message
  : typeof error === 'object' && error && 'message' in error ? String(error.message) : String(error);

export function IntegrationSettings({ kind }: { kind: keyof typeof tools }) {
  const tool = tools[kind];
  const [status, setStatus] = useState<ManagedToolStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [uninstall, setUninstall] = useState(false);
  const refresh = async () => {
    setBusy(true); setError(null);
    try { setStatus(await tool.status()); }
    catch (e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  };
  useEffect(() => { void refresh(); }, [kind]);
  const manage = async (operation: 'install' | 'upgrade' | 'uninstall') => {
    setBusy(true); setError(null);
    try {
      setStatus(await tool.manage(operation));
      toast(`${tool.name} ${operation === 'uninstall' ? '已卸载' : operation === 'upgrade' ? '已升级' : '已安装'}`);
    } catch (e) {
      setError(errorMessage(e));
      try { setStatus(await tool.status()); } catch { setStatus(null); }
      throw e;
    } finally { setBusy(false); }
  };
  return <div className="settings-section cli-settings"><h2>{tool.title}</h2><p>{tool.description}</p>
    <div className="cli-status" aria-live="polite"><span>{status ? status.installed ? `已安装 ${status.version ?? '（版本不可用）'}` : '未安装' : busy ? '正在读取状态…' : '状态读取失败'}</span>{status ? <span>App 内置 {status.bundledVersion}</span> : null}</div>
    <code className="cli-path">{status?.path ?? tool.path}</code>
    {error || status?.message ? <p className="cli-message">{error ?? status?.message}</p> : null}
    <div className="backup-actions"><Button disabled={busy || !status?.canInstall} onClick={() => void run(() => manage('install'))}>安装</Button><Button disabled={busy || !status?.canUpgrade} onClick={() => void run(() => manage('upgrade'))}>升级到 App 版本</Button><Button disabled={busy || !status?.canUninstall} onClick={() => setUninstall(true)}>卸载</Button><IconButton icon="retry" label={`刷新 ${tool.name} 状态`} disabled={busy} onClick={() => void refresh()}/></div>
    <p className="cli-hint">{tool.hint}</p>
    {uninstall ? <Confirm title={`卸载${tool.title}？`} detail={tool.uninstall} onClose={() => setUninstall(false)} onConfirm={() => manage('uninstall')}/> : null}
  </div>;
}
