# Demo — Slice 8b: Groups and Customize

You can bundle widgets into your own titled groups, and copy a platform dashboard into an
editable one. Placement is automatic; there is no drag or resize (T-011).

## Setup

Same as [08-user-dashboards](08-user-dashboards.md): fresh app data, `/tmp/platform` connected
and approved, then `npm run dev`.

## Steps

1. **Create a group.** Open **+ New dashboard**, name it "Mine", then click **+ New group** in
   the toolbar and title it "Ops". A dashed-edged card titled **Ops** appears with "Nothing in
   this group yet." and an **Add widget** button, plus **Rename** and **Remove** in its header.
2. **Add two widgets into it.** Click **Add widget** inside the Ops card (not the toolbar one).
   Pick a widget; it appears inside Ops. Repeat with a second. Both render in Ops's sub-grid.
3. **Rename it.** Click **Rename** in the Ops header, change the title to "On-call", confirm.
   The header and the sub-grid heading update.
4. **Remove one member.** Click **Remove** on one member. A dialog opens with **Cancel** first;
   confirm. Only that member disappears; the group and the other member stay.
5. **Customize a platform dashboard.** Open a platform tab (for example Home). The toolbar shows
   only **+ New dashboard** and **Customize**. Click **Customize**: the app opens a new user
   dashboard with the same widgets, now editable (Add widget, + New group, Rename, Delete).
6. **Persistence.** Quit and reopen the app. Your groups, members, titles and the customized copy
   are all still there. Removing a group removes its members with it.
