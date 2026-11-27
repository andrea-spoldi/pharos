/** Test helpers shared across suites. */

/** Sets the window width and fires the resize event `viewport.ts` listens for. */
export function setViewport(width: number) {
  window.innerWidth = width;
  window.dispatchEvent(new Event("resize"));
}
