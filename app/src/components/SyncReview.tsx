import { For, Show } from "solid-js";
import type { Capabilities, Changes, Diagnostic, SyncPlan } from "../bindings";

export const shortCommit = (sha: string) => sha.slice(0, 7);

const CAPABILITY_LABELS: [keyof Capabilities, string][] = [
  ["hosts", "Call"],
  ["executables", "Run"],
  ["paths", "Read"],
];

/** True when `c` declares anything. */
export const hasAnyCapability = (c: Capabilities) => CAPABILITY_LABELS.some(([key]) => c[key]?.length);

/** The Call/Run/Read rows of a capability set, as `<dd>`s inside a `<dl>`. */
export function CapabilityList(props: { caps: Capabilities }) {
  return (
    <For each={CAPABILITY_LABELS.filter(([key]) => props.caps[key]?.length)}>
      {([key, verb]) => (
        <dd>
          {verb} <code>{props.caps[key]!.join(", ")}</code>
        </dd>
      )}
    </For>
  );
}

export function ChangeLine(props: { label: string; changes: Changes }) {
  const parts = () =>
    [
      [props.changes.added, "added"],
      [props.changes.changed, "changed"],
      [props.changes.removed, "removed"],
    ] as const;
  return (
    <li>
      <strong>{props.label}:</strong>{" "}
      <For each={parts().filter(([names]) => names.length > 0)} fallback={<span class="muted">no changes</span>}>
        {([names, verb], i) => (
          <span>
            {i() > 0 && "; "}
            {verb} {names.join(", ")}
          </span>
        )}
      </For>
    </li>
  );
}

function DiagnosticItem(props: { d: Diagnostic }) {
  return (
    <li>
      <code>
        {props.d.file}
        {props.d.line != null && `:${props.d.line}`}
        {props.d.path && ` ${props.d.path}`}
      </code>{" "}
      {props.d.message} <span class="muted">[{props.d.code}]</span>
    </li>
  );
}

/** Review of a plan held by the engine. Applying approves exactly that plan (D-014). */
export function SyncReview(props: {
  plan: SyncPlan;
  firstSync: boolean;
  busy: boolean;
  onApply: (consent: boolean) => void;
  onDiscard: () => void;
  onReview: () => void;
}) {
  const errors = () => props.plan.diagnostics.filter((d) => d.severity === "error");
  const warnings = () => props.plan.diagnostics.filter((d) => d.severity === "warning");
  const added = () => Object.entries(props.plan.capability_changes).filter(([, c]) => hasAnyCapability(c.added));
  const removed = () => Object.entries(props.plan.capability_changes).filter(([, c]) => hasAnyCapability(c.removed));

  return (
    <section class="stack" aria-labelledby="review-title">
      <header>
        <h2 id="review-title">{props.firstSync ? "Review the platform repo" : "Review changes"}</h2>
        <p class="muted">
          Commit <code>{shortCommit(props.plan.to_commit)}</code>
          <Show when={props.plan.from_commit}>
            {(from) => (
              <>
                {" "}
                (applied: <code>{shortCommit(from())}</code>)
              </>
            )}
          </Show>
        </p>
      </header>

      <ul class="plain">
        <ChangeLine label="Widgets" changes={props.plan.widgets} />
        <ChangeLine label="Dashboards" changes={props.plan.dashboards} />
        <Show when={props.plan.platform_changed}>
          <li>
            <strong>Platform settings</strong> changed
          </li>
        </Show>
      </ul>

      <Show when={errors().length > 0}>
        <div class="notice danger" role="alert">
          <strong>
            {errors().length} lint error{errors().length === 1 ? "" : "s"}: this version can't be applied
          </strong>
          <ul class="diagnostics">
            <For each={errors()}>{(d) => <DiagnosticItem d={d} />}</For>
          </ul>
          <p class="muted">
            Ask the platform team to fix the repo (<code>pharos lint</code> shows the same list), then
            review again.
          </p>
        </div>
      </Show>

      <Show when={warnings().length > 0}>
        <div class="notice">
          <strong>
            {warnings().length} warning{warnings().length === 1 ? "" : "s"}
          </strong>
          <ul class="diagnostics">
            <For each={warnings()}>{(d) => <DiagnosticItem d={d} />}</For>
          </ul>
        </div>
      </Show>

      <Show when={errors().length === 0 && added().length > 0}>
        <div class="notice" data-testid="capabilities">
          <strong>New permissions</strong>
          <p class="muted">
            They run locally with your credentials. A widget you don't approve stays blocked until you approve it
            on its card.
          </p>
          <dl class="capabilities">
            <For each={added()}>
              {([widget, change]) => (
                <>
                  <dt>{widget}</dt>
                  <CapabilityList caps={change.added} />
                </>
              )}
            </For>
          </dl>
        </div>
      </Show>

      <Show when={errors().length === 0 && removed().length > 0}>
        <div class="notice" data-testid="capabilities-removed">
          <strong>No longer needed</strong>
          <dl class="capabilities muted">
            <For each={removed()}>
              {([widget, change]) => (
                <>
                  <dt>{widget}</dt>
                  <CapabilityList caps={change.removed} />
                </>
              )}
            </For>
          </dl>
        </div>
      </Show>

      <div class="row">
        <Show
          when={errors().length === 0}
          fallback={
            <button type="button" class="primary" disabled={props.busy} onClick={() => props.onReview()}>
              Review again
            </button>
          }
        >
          <Show
            when={props.plan.needs_consent}
            fallback={
              <button type="button" class="primary" disabled={props.busy} onClick={() => props.onApply(false)}>
                Apply
              </button>
            }
          >
            <button type="button" class="primary" disabled={props.busy} onClick={() => props.onApply(true)}>
              Approve and apply
            </button>
            <button type="button" disabled={props.busy} onClick={() => props.onApply(false)}>
              Apply, keep these blocked
            </button>
          </Show>
        </Show>
        <button type="button" disabled={props.busy} onClick={() => props.onDiscard()}>
          Discard
        </button>
      </div>
    </section>
  );
}
