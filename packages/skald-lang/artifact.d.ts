/**
 * Node-only helpers for portable `.skald` sidecars, manifests, and receipts.
 * These APIs read and write files; they are not a browser export.
 */

export const ARTIFACT_FORMAT_VERSION: 2;
export const ARTIFACT_FORMAT_LEGACY: 1;
export const RECEIPT_FORMAT_VERSION: 2;
export const RECEIPT_FORMAT_LEGACY: 1;

export type SeedRecord = { type: string; value: string };

export interface ArtifactDependency {
  path: string;
  hash: string;
  role?: string;
}

export interface ArtifactManifest {
  formatVersion: number;
  runtimeVersion: string;
  runProfile: string;
  locale: string;
  patternHash: string;
  seed?: SeedRecord;
  case?: string;
  nsfw: boolean;
  story: boolean;
  dependencies?: ArtifactDependency[];
  dictionaryHash: string;
  dictOnly?: boolean;
}

export interface ArtifactReceipt {
  formatVersion: number;
  runProfile: string;
  patternHash: string;
  text: string;
  channels?: Record<string, string>;
  seed?: SeedRecord;
}

export interface ReceiptCheck {
  replayed: boolean;
  legacy: boolean;
}

export function sha256Hex(bytes: string | Uint8Array): string;
export function patternHash(pattern: string): string;
export function fileHash(bytes: string | Uint8Array): string;
export function sidecarPath(patternPath: string): string;
export function normalizeSeed(seed?: unknown): SeedRecord | null;
export function seedRecord(seed?: unknown): SeedRecord | null;
export function receiptPath(
  patternPath: string,
  runSeed?: unknown,
  manifestSeed?: unknown,
): string;
export function resolveDependencyPath(
  baseDir: string | undefined | null,
  depPath: string,
): string;
export function storedDependencyPath(artifactPath: string, given: string): string;
export function looksLikeLanguagePackText(src: string): boolean;

export function manifestForPattern(
  pattern: string,
  options?: {
    seed?: unknown;
    caseMode?: string;
    nsfw?: boolean;
    story?: boolean;
    runtimeVersion?: string;
    locale?: string;
    dictionaryJson?: string | object;
    dependencies?: ArtifactDependency[];
    dictOnly?: boolean;
  },
): ArtifactManifest;

export function writeManifest(path: string, manifest: ArtifactManifest): void;
export function writeReceipt(path: string, receipt: ArtifactReceipt | object): void;
export function readManifest(
  path: string,
  options?: { runtimeVersion?: string },
): ArtifactManifest;
export function replayLocked(manifest: ArtifactManifest): boolean;
export function verifyPattern(pattern: string, manifest: ArtifactManifest): void;
export function verifyLock(
  manifest: ArtifactManifest,
  dictionaryJson?: string | object | null,
  options?: { baseDir?: string },
): void;
export function readReceipt(path: string): ArtifactReceipt;
export function verifyReceipt(
  receipt: ArtifactReceipt,
  text: string,
  pattern: string,
  channels?: Record<string, string>,
): ReceiptCheck;
export function receiptExists(patternPath: string): boolean;
