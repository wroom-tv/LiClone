import { open } from "@tauri-apps/plugin-dialog";

export async function chooseFolder() {
  const picked = await open({
    directory: true,
    multiple: false,
    title: "Choose a folder",
  });
  return typeof picked === "string" ? picked : null;
}
