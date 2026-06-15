/**
 * Syncora Design Tokens — aligned with XEnsemble / ParaRouter Console surface.
 *
 * Palette: zinc-* (no gray-* / blue-* for chrome)
 * Accent: black / zinc (no purple / marketing colors)
 * Radius: controls rounded-md, cards/tables/dialogs rounded-lg
 * Shadow: cards/tables/dialogs shadow-sm
 */

/* ---- Backdrop ---- */
export const backdropClass = "fixed inset-0 bg-black/50 transition-opacity";

/* ---- Dialog ---- */
export const dialogPanelBaseClass =
  "relative bg-white border border-zinc-200 shadow-sm rounded-lg text-left flex flex-col overflow-hidden";

/** sm — confirm / short single-step (384px) */
export const dialogSmClass =
  `${dialogPanelBaseClass} w-full max-w-sm max-w-[calc(100vw-2rem)]`;

/** md — standard forms (480px) */
export const dialogMdClass =
  `${dialogPanelBaseClass} w-[480px] max-w-[calc(100vw-2rem)]`;

/** lg — multi-section long forms (560px) */
export const dialogLgClass =
  `${dialogPanelBaseClass} w-[560px] max-w-[calc(100vw-2rem)]`;

/** Structured dialog shell (Header / Body / Footer) */
export const structuredDialogPanelClass =
  `${dialogMdClass} flex flex-col max-h-[90vh] overflow-hidden p-0`;

export const structuredDialogHeaderClass =
  "px-5 py-4 border-b border-zinc-200 shrink-0 bg-white";

export const structuredDialogBodyClass =
  "flex-1 min-h-0 overflow-y-auto px-5 py-5 space-y-4";

export const structuredDialogFooterClass =
  "border-t border-zinc-200 px-6 py-4 bg-zinc-50/80 flex justify-end gap-2 shrink-0";

/* ---- Input ---- */
export const inputClass =
  "w-full bg-white border border-zinc-300 rounded-md px-3 py-2 text-sm text-zinc-900 placeholder:text-zinc-400 focus:outline-none focus:border-black focus:ring-1 focus:ring-black transition-colors";

/* ---- Form label ---- */
export const formLabelClass =
  "block text-xs font-semibold uppercase tracking-wider text-zinc-500";

/* ---- Page ---- */
export const pageStackClass = "space-y-6";
export const pageTitleClass = "text-2xl font-bold tracking-tight text-zinc-900";
export const pageSubtitleClass = "text-sm text-zinc-500";

/* ---- Card ---- */
export const cardClass = "bg-white border border-zinc-200 rounded-lg shadow-sm";

/* ---- Table ---- */
export const tableShellClass =
  "bg-white border border-zinc-200 rounded-lg overflow-hidden shadow-sm";
export const tableHeadRowClass = "bg-zinc-50 border-b border-zinc-200";
export const tableBodyDivideClass = "divide-y divide-zinc-200";
export const tableBodyRowClass = "hover:bg-zinc-50/50 transition-colors";
export const tableHeadCellClass =
  "px-4 py-2.5 text-xs font-semibold text-zinc-500 uppercase tracking-wider";
export const tableBodyCellClass = "px-4 py-3 text-sm text-zinc-700";

/* ---- Status badge (fixed slot) ---- */
export const statusBadgeClass =
  "inline-flex items-center gap-1 min-w-[5rem] h-5 text-xs font-medium rounded-full px-2";

/* ---- Empty state ---- */
export const emptyStateClass =
  "flex flex-col items-center justify-center rounded-lg border border-dashed border-zinc-300 bg-zinc-50/50 py-12 px-6";

/* ---- Nav ---- */
export const navActiveClass = "bg-zinc-100 text-zinc-900";
export const navIdleClass = "text-zinc-600 hover:bg-zinc-50 hover:text-zinc-900";

/* ---- Icon button (table row / toolbar) ---- */
export const iconButtonClass =
  "inline-flex items-center justify-center rounded-md p-1.5 text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900 disabled:opacity-40 disabled:pointer-events-none focus:outline-none focus:ring-2 focus:ring-black focus:ring-offset-1";

export const iconButtonDangerClass =
  "inline-flex items-center justify-center rounded-md p-1.5 text-red-500 hover:bg-red-50 hover:text-red-700 disabled:opacity-40 disabled:pointer-events-none focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-1";

/* ---- Stat ---- */
export const statValueClass = "text-2xl font-bold tracking-tight text-zinc-900";
