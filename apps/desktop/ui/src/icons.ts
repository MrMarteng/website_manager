// Outline icons from @tabler/icons (MIT), bundled at build time – no web fonts, no CDN.
import alert from "@tabler/icons/outline/alert-triangle.svg?raw";
import arrowRight from "@tabler/icons/outline/arrow-right.svg?raw";
import check from "@tabler/icons/outline/check.svg?raw";
import chevronRight from "@tabler/icons/outline/chevron-right.svg?raw";
import circleCheck from "@tabler/icons/outline/circle-check.svg?raw";
import cloudUpload from "@tabler/icons/outline/cloud-upload.svg?raw";
import code from "@tabler/icons/outline/code.svg?raw";
import dashboard from "@tabler/icons/outline/layout-dashboard.svg?raw";
import download from "@tabler/icons/outline/download.svg?raw";
import edit from "@tabler/icons/outline/edit.svg?raw";
import externalLink from "@tabler/icons/outline/external-link.svg?raw";
import eye from "@tabler/icons/outline/eye.svg?raw";
import file from "@tabler/icons/outline/file.svg?raw";
import filePlus from "@tabler/icons/outline/file-plus.svg?raw";
import fileText from "@tabler/icons/outline/file-text.svg?raw";
import folder from "@tabler/icons/outline/folder.svg?raw";
import folderOpen from "@tabler/icons/outline/folder-open.svg?raw";
import hammer from "@tabler/icons/outline/hammer.svg?raw";
import history from "@tabler/icons/outline/history.svg?raw";
import info from "@tabler/icons/outline/info-circle.svg?raw";
import key from "@tabler/icons/outline/key.svg?raw";
import loader from "@tabler/icons/outline/loader-2.svg?raw";
import lock from "@tabler/icons/outline/lock.svg?raw";
import playerPlay from "@tabler/icons/outline/player-play.svg?raw";
import playerStop from "@tabler/icons/outline/player-stop.svg?raw";
import refresh from "@tabler/icons/outline/refresh.svg?raw";
import restore from "@tabler/icons/outline/restore.svg?raw";
import settings from "@tabler/icons/outline/settings.svg?raw";
import shield from "@tabler/icons/outline/shield-check.svg?raw";
import terminal from "@tabler/icons/outline/terminal-2.svg?raw";
import trash from "@tabler/icons/outline/trash.svg?raw";
import upload from "@tabler/icons/outline/upload.svg?raw";
import world from "@tabler/icons/outline/world.svg?raw";
import x from "@tabler/icons/outline/x.svg?raw";

export const icons = {
  alert,
  arrowRight,
  check,
  chevronRight,
  circleCheck,
  cloudUpload,
  code,
  dashboard,
  download,
  edit,
  externalLink,
  eye,
  file,
  filePlus,
  fileText,
  folder,
  folderOpen,
  hammer,
  history,
  info,
  key,
  loader,
  lock,
  playerPlay,
  playerStop,
  refresh,
  restore,
  settings,
  shield,
  terminal,
  trash,
  upload,
  world,
  x,
} as const;

export type IconName = keyof typeof icons;
