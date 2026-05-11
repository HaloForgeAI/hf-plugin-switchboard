import type { TargetStatus } from "../types";

export function TargetCard({ target }: { target: TargetStatus }) {
  return (
    <article className="sb-target-card">
      <div className="sb-target-head">
        <div>
          <h2>{target.label}</h2>
          {target.summary && <p>{target.summary}</p>}
        </div>
        <span className={target.configured ? "sb-badge sb-badge-on" : "sb-badge"}>
          {target.configured ? "Configured" : "Empty"}
        </span>
      </div>
      <div className="sb-path-list">
        {target.paths.map((path) => (
          <div key={`${target.id}:${path.label}`} className="sb-path-row">
            <span>{path.label}</span>
            <code>{path.path}</code>
            <i className={path.exists ? "sb-dot sb-dot-on" : "sb-dot"} />
          </div>
        ))}
      </div>
    </article>
  );
}
