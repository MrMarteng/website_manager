// Data exchanged with the Rust commands (see apps/desktop/src-tauri/src/*.rs, camelCase via serde).

export interface VersionInfo {
  id: string;
  version: string;
  created: string;
  changelog: string[];
  current: boolean;
}

export interface ProjectSettings {
  protocol?: string | null;
  host?: string | null;
  user?: string | null;
  port?: number | null;
  remoteRoot?: string | null;
  domain?: string | null;
  mappings?: string | null;
}

export interface MappingInfo {
  source: string;
  target: string;
  mode: "sync" | "copy" | "seed";
  protect: string[];
}

export interface Detected {
  protocol: "sftp" | "ftps";
  host: string;
  user: string;
  port: number;
  root: string;
  mappings: MappingInfo[];
  hostKeyPinned: boolean;
}

export interface LastUpload {
  version: string;
  created: string;
  uploadedAt: string;
  target: "server" | "folder";
  destination: string;
}

export interface Overview {
  name: string;
  publisher: string;
  current: VersionInfo;
  versions: VersionInfo[];
  settings: ProjectSettings;
  detected: Detected | null;
  deployError: string | null;
  storedCredentials: boolean;
  packageHasCredentials: boolean;
  hasCompliance: boolean;
  hasSources: boolean;
  canBuild: boolean;
  lastUpload: LastUpload | null;
}

export interface OpenResult {
  overview: Overview;
  firstUse: boolean;
  previousVersion: string | null;
  locallyChanged: string[];
}

export interface ProjectListItem {
  name: string;
  version: string;
}

export type UploadTarget =
  | { kind: "server"; password: string | null }
  | { kind: "localFolder"; folder: string };

export interface UploadRequest {
  target: UploadTarget;
  dryRun: boolean;
  allowOlder: boolean;
  acceptUnverifiedHost: boolean;
}

export interface UploadResult {
  dryRun: boolean;
  done: number;
  total: number;
  skippedSeed: number;
  uploadedCount: number;
  deletedCount: number;
  uploaded: string[];
  deleted: string[];
  website: string | null;
}

export interface FileEntry {
  path: string;
  size: number;
}

export interface ServerStatus {
  preview: string | null;
  dev: string | null;
}

/** Error as sent by the commands (`CommandError` in Rust). */
export interface CommandError {
  code: string | null;
  detail: string;
}
