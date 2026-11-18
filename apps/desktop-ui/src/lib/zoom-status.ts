// Zoom connection status from the companion (`GET /api/zoom`, WS `zoom_status`).
export type ZoomStatus = { state: string; reason?: string; detail?: string };
type T = (zh: string, en: string) => string;
export type ZoomAction = 'sign_in' | 'reopen_sign_in' | 'switch_account';
export type ZoomBanner = { tone: 'info' | 'warn' | 'error'; title: string; body: string; action: ZoomAction | null; detail?: string };

/** States in which local tokens exist, so "Sign out of Zoom" applies. */
export function zoomSignedIn(s: ZoomStatus | null): boolean {
  return !!s && ['connected', 'connecting', 'zoom_not_running', 'error'].includes(s.state) && s.reason !== 'missing_scope';
}

export function zoomSidebarLabel(s: ZoomStatus | null, bridgeUp: boolean, t: T): string {
  if (!bridgeUp || !s) return t('正在启动', 'Starting');
  switch (s.state) {
    case 'connected': return t('已连接 Zoom', 'Connected to Zoom');
    case 'signing_in': return t('正在登录', 'Signing in');
    case 'signed_out': return t('未登录 Zoom', 'Not signed in');
    case 'zoom_not_running': return t('Zoom 未运行', 'Zoom is not running');
    case 'not_configured': return t('未配置', 'Not configured');
    case 'error': return t('连接出现问题', 'Connection problem');
    default: return t('正在连接 Zoom', 'Connecting to Zoom');
  }
}

/** What the top-of-page banner says; null when attendance capture is working. */
export function zoomBanner(s: ZoomStatus | null, t: T): ZoomBanner | null {
  if (!s) return null;
  const sp = t('', ' '); // sentence joiner: none in Chinese
  const sameAccount = t('请使用与 Zoom 客户端相同的 Zoom 账号登录。', 'Use the same Zoom account that is signed in to the Zoom app.');
  switch (s.state) {
    case 'connected':
      return null;
    case 'signed_out': {
      if (s.reason === 'session_expired') return { tone: 'warn', title: t('Zoom 登录已过期', 'Your Zoom sign-in has expired'), body: t('请重新登录以继续记录考勤。', 'Sign in again to keep capturing attendance.') + sp + sameAccount, action: 'sign_in' };
      if (s.reason === 'sign_in_denied') return { tone: 'warn', title: t('登录已取消', 'Sign-in was cancelled'), body: t('需要在浏览器中允许访问，AttendFlow Desktop 才能读取会议参与者。', 'AttendFlow Desktop needs your permission in the browser to read meeting participants.'), action: 'sign_in' };
      if (s.reason === 'sign_in_timeout' || s.reason === 'sign_in_failed') return { tone: 'warn', title: t('登录未完成', 'Sign-in did not finish'), body: t('请重试。', 'Please try again.') + sp + sameAccount, action: 'sign_in', detail: s.detail };
      return { tone: 'info', title: t('使用 Zoom 登录', 'Sign in with Zoom'), body: t('登录后，AttendFlow Desktop 会在你参加会议时自动记录考勤。', 'After you sign in, AttendFlow Desktop records attendance automatically whenever you are in a meeting.') + sp + sameAccount, action: 'sign_in' };
    }
    case 'signing_in':
      return { tone: 'info', title: t('请在浏览器中完成登录', 'Finish signing in in your browser'), body: t('浏览器中已打开 Zoom 登录页面。完成后回到这里即可。', 'A Zoom sign-in page opened in your browser. Come back here when you are done.'), action: 'reopen_sign_in' };
    case 'zoom_not_running':
      return { tone: 'warn', title: t('请打开 Zoom 客户端', 'Open the Zoom app'), body: t('启动 Zoom Workplace 并登录后，AttendFlow Desktop 会自动连接。', 'Start Zoom Workplace and sign in. AttendFlow Desktop connects automatically.'), action: null };
    case 'not_configured':
      return { tone: 'error', title: t('此版本缺少 Zoom 应用配置', 'This build has no Zoom app configured'), body: t('请安装正式发布的版本。', 'Please install the released version of AttendFlow Desktop.'), action: null, detail: s.detail };
    case 'error':
      if (s.reason === 'account_mismatch') return { tone: 'error', title: t('Zoom 账号不一致', 'Different Zoom accounts'), body: t('Zoom 客户端登录的账号与 AttendFlow Desktop 不同。请在 Zoom 客户端中切换账号，或在此使用另一个账号登录。', 'The Zoom app is signed in with a different account than AttendFlow Desktop. Switch accounts in the Zoom app, or sign in here with the other account.'), action: 'switch_account' };
      if (s.reason === 'missing_scope') return { tone: 'error', title: t('缺少权限', 'Permission missing'), body: t('Zoom 没有授予所需的权限。请重新登录并允许访问。', 'Zoom did not grant the permission AttendFlow Desktop needs. Sign in again and allow access.'), action: 'switch_account', detail: s.detail };
      if (s.reason === 'ipc_timeout') return { tone: 'warn', title: t('Zoom 客户端没有响应', 'The Zoom app did not respond'), body: t('请确认 Zoom 客户端已打开，并且登录的账号与 AttendFlow Desktop 相同。正在自动重试。', 'Make sure the Zoom app is open and signed in with the same Zoom account as AttendFlow Desktop. Retrying automatically.'), action: 'switch_account', detail: s.detail };
      if (s.reason === 'network') return { tone: 'warn', title: t('无法连接到 Zoom', 'Cannot reach Zoom'), body: t('请检查网络连接。正在自动重试。', 'Check your internet connection. Retrying automatically.'), action: null, detail: s.detail };
      return { tone: 'warn', title: t('无法连接到 Zoom 客户端', 'Cannot connect to the Zoom app'), body: t('正在自动重试。如果问题持续，请重新启动 Zoom 客户端。', 'Retrying automatically. If this continues, restart the Zoom app.'), action: null, detail: s.detail };
    default:
      return { tone: 'info', title: t('正在连接 Zoom 客户端', 'Connecting to the Zoom app'), body: t('请稍候。', 'One moment.'), action: null };
  }
}

export function zoomActionLabel(action: ZoomAction, t: T): string {
  if (action === 'reopen_sign_in') return t('重新打开登录页面', 'Open sign-in page again');
  if (action === 'switch_account') return t('使用其他账号登录', 'Sign in with another account');
  return t('使用 Zoom 登录', 'Sign in with Zoom');
}
