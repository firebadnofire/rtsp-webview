export const renderEmptyWorkspace = (): string => {
  return `<section class="empty-workspace">
    <h2>No Screens Configured</h2>
    <p>Start by creating your first camera manually or run bulk autoconfiguration.</p>
    <div class="empty-workspace-actions">
      <button data-action="empty-manual-setup">Create First Screen</button>
      <button data-action="empty-open-auto-populate">Run Bulk Autoconfiguration</button>
    </div>
  </section>`
}
