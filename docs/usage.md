# Using LiClone

LiClone keeps a window for setup and a tray icon for everyday use. Closing the window does not quit the app. A notification tells you it is still in the tray. Click the tray icon to open the window again. Choose **Quit** on that icon when you want LiClone to stop.

If **Start LiClone when I sign in** is on, the next sign-in starts LiClone in the tray. The window stays hidden until you open it.

## rclone

If the header says rclone is missing, choose **Install rclone**. LiClone installs it for the current user and adds that folder to your user PATH. A terminal you open after that can see rclone. LiClone itself does not need a reboot.

The header names the rclone version. LiClone expects 1.65 or newer. Older rclone can reject a remote edit or a mount setting.

A drive letter needs [WinFSP](https://winfsp.dev/). Install WinFSP, then start the mount again if the drive does not appear.

## First save

When you save a mount, LiClone confirms the name. If remote control is on, the note also shows the port, for example port 5573.

## Remotes

Open **Remotes**, select one, and choose **Edit**.

Saved settings are listed first. Leave a password or token blank to keep the value rclone already has. **Reconnect** starts the browser sign-in again. **Add another field** shows settings this remote does not have yet.

Passwords and tokens stay in rclone’s own config. LiClone does not keep a second copy.

## Mounts

A mount profile is a saved `rclone mount`. You can start it from LiClone, and you can turn on **Start when I log in** so that mount comes back after you sign in to Windows.

**Edit** on a saved profile opens it again. Clicking the row does the same.

**Add to Mounts** on a running rclone mount copies that process into a profile, including the flags it was started with. If that drive letter already has a profile, the button is **Update mount**.

LiClone will not start a second mount on a drive letter that is already in use, or on the same remote and folder that is already mounted. The button stays on **Mounted** until that one stops.

### Presets and settings

A highlighted preset is the one your current settings match. Everyday, video, editing, browse-only, and low disk use are starting points. You can change any setting after you pick one.

Most values are a slider or a short list of choices. An empty choice shows its meaning, such as **Default**, **No limit**, or **Off**. **Extra flags** is for a raw rclone argument that is not in the form. **Browse** picks a folder for the mount point or the cache directory.

### Cache size

For a new mount, LiClone suggests a cache size limit: 10% of the free space on the disk where your user profile lives, rounded to a whole gigabyte. The suggestion is at least 1 GB and never more than 100 GB. The hint under the setting states the number for this PC. A preset that sets its own size keeps that size. If you already chose **No limit** or typed a size, LiClone leaves your choice alone.

### Remote control

Remote control lets LiClone ask a running mount which file is uploading. Each profile gets its own port, starting at 5572. If that port is already used by another saved mount or by rclone, LiClone moves this profile to the next free port and tells you which one it saved.

Leave remote control on unless you have a reason to turn it off. Without it, LiClone can still list files waiting to upload when the mount cache mode is **writes** or **full**, using rclone’s own cache notes. It cannot show a live “Uploading” line for that mount.

## Transfers

A file is listed only while rclone still treats it as an upload: the bytes are on this PC and have not finished going to the cloud. Opening a file so it can be read does not put it here.

- **Saving to disk** — the local copy is still growing
- **Waiting to upload** — the file is on disk and the upload has not started
- **Uploading** — remote control reports that this file is going to the cloud

## Clear cache

**Clear cache** always shows a preview first. Nothing is deleted until you confirm.

- **Read cache that is not an upload** — files rclone downloaded for reading, not files you changed
- **Cached files already in the cloud** — files whose size matches the cloud. LiClone checks each cached remote with `rclone check --size-only --one-way`
- **Untouched read cache** — read cache that has not been used for the number of days you set

Files that are still waiting to upload stay on disk. That includes a file that becomes an upload while the preview is open.

The sizes in the preview are the file sizes rclone stored, which is what the cache page shows. They are not the smaller amount of disk space a sparse cache file may be using.

## Running mounts

**Instances** lists rclone processes LiClone can see, including ones you started outside the app. You can stop one from that list. Stopping a mount unmounts that drive. Wait until Windows has released the drive letter before you start it again.
