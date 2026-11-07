import sharp from "sharp";
import { createHash } from "node:crypto";

export const maxAvatarUpload = 5_000_000;
export type Crop = { left: number; top: number; size: number };
export class AvatarImageError extends Error {
  constructor(public code: string, public status = 400) { super(code); }
}
let active = 0;
export async function withAvatarSlot<T>(work: () => Promise<T>): Promise<T> {
  if (active >= 2) throw new AvatarImageError("image_processor_busy", 429);
  active++;
  try { return await work(); } finally { active--; }
}
export async function readAvatarBody(request: Request): Promise<Buffer> {
  const reader = request.body?.getReader(); if (!reader) throw new AvatarImageError("image_required");
  let size = 0; const chunks: Uint8Array[] = []; const deadline = Date.now() + 7000;
  try {
    while (true) {
      const remaining = deadline - Date.now(); if (remaining <= 0) throw new AvatarImageError("upload_timeout", 408);
      let timer: ReturnType<typeof setTimeout> | undefined;
      const part = await Promise.race([reader.read(), new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new AvatarImageError("upload_timeout", 408)), remaining); })]).finally(() => clearTimeout(timer));
      if (part.done) break; size += part.value.length;
      if (size > maxAvatarUpload) throw new AvatarImageError("image_exceeds_5_mb", 413);
      chunks.push(part.value);
    }
  } catch (error) { void reader.cancel().catch(() => {}); throw error; }
  finally { reader.releaseLock(); }
  return Buffer.concat(chunks);
}
function pngChunks(bytes: Buffer, criticalOnly: boolean): Buffer {
  const chunks = [bytes.subarray(0, 8)]; let offset = 8, count = 0;
  while (offset < bytes.length) {
    if (++count > 1024 || offset + 12 > bytes.length) throw new AvatarImageError("malformed_png");
    const size = bytes.readUInt32BE(offset), end = offset + size + 12;
    if (end > bytes.length) throw new AvatarImageError("malformed_png");
    const kind = bytes.subarray(offset + 4, offset + 8).toString("ascii");
    if (kind === "acTL") throw new AvatarImageError("animated_images_unavailable");
    if (!criticalOnly || ["IHDR", "IDAT", "IEND"].includes(kind)) chunks.push(bytes.subarray(offset, end));
    offset = end;
  }
  return Buffer.concat(chunks);
}
export async function normalizeAvatar(bytes: Buffer, mime: string, crop?: Crop): Promise<Buffer> {
  if (!bytes.length) throw new AvatarImageError("image_required");
  if (bytes.length > maxAvatarUpload) throw new AvatarImageError("image_exceeds_5_mb", 413);
  const formats: Record<string, string> = { "image/png": "png", "image/jpeg": "jpeg", "image/webp": "webp" };
  const expected = formats[mime]; if (!expected) throw new AvatarImageError("png_jpeg_webp_required", 415);
  const signature = expected === "png" ? bytes.subarray(0, 8).equals(Buffer.from("89504e470d0a1a0a", "hex"))
    : expected === "jpeg" ? bytes[0] === 255 && bytes[1] === 216 && bytes[2] === 255
    : bytes.subarray(0, 4).toString() === "RIFF" && bytes.subarray(8, 12).toString() === "WEBP";
  if (!signature) throw new AvatarImageError("image_content_type_mismatch", 415);
  if (expected === "png") pngChunks(bytes, false);
  try {
    const image = sharp(bytes, { limitInputPixels: 16_777_216, failOn: "warning", sequentialRead: true });
    const info = await image.metadata();
    if (info.format !== expected || (info.pages ?? 1) !== 1 || !info.width || !info.height || Math.max(info.width, info.height) > 4096) throw new AvatarImageError("image_dimensions_or_animation_invalid");
    const swap = (info.orientation ?? 1) >= 5;
    const width = swap ? info.height : info.width, height = swap ? info.width : info.height;
    const square = crop ?? { left: Math.floor((width - Math.min(width, height)) / 2), top: Math.floor((height - Math.min(width, height)) / 2), size: Math.min(width, height) };
    if (![square.left, square.top, square.size].every(Number.isSafeInteger) || square.left < 0 || square.top < 0 || square.size < 1 || square.left + square.size > width || square.top + square.size > height) throw new AvatarImageError("invalid_square_crop");
    const encoded = await image.rotate().extract({ left: square.left, top: square.top, width: square.size, height: square.size }).resize(512, 512).toColourspace("srgb").ensureAlpha().png({ compressionLevel: 9, adaptiveFiltering: false, palette: false }).timeout({ seconds: 5 }).toBuffer();
    const clean = pngChunks(encoded, true);
    if (clean.length > 1_100_000) throw new AvatarImageError("normalized_image_too_large", 413);
    return clean;
  } catch (error) { if (error instanceof AvatarImageError) throw error; throw new AvatarImageError("image_decode_failed"); }
}
export async function defaultAvatar(seed: string): Promise<Buffer> {
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(seed)) throw new AvatarImageError("invalid_avatar_seed");
  const hash = createHash("sha256").update(`supa-avatar-v1:${seed.toLowerCase()}`).digest();
  const colors = [[222, 217, 189], [113, 149, 125], [205, 121, 110]];
  const raw = Buffer.alloc(7 * 7 * 3); for (let y = 0; y < 7; y++) for (let x = 0; x < 7; x++) {
    const colored = x > 0 && x < 6 && y > 0 && y < 6 && hash[(y - 1) * 3 + Math.min(x - 1, 5 - x)] >= 128;
    const color = colored ? colors[hash[31] % colors.length] : [8, 9, 8];
    raw.set(color, (y * 7 + x) * 3);
  }
  return pngChunks(await sharp(raw, { raw: { width: 7, height: 7, channels: 3 } }).resize(512, 512, { kernel: "nearest" }).ensureAlpha().png({ compressionLevel: 9, palette: false }).toBuffer(), true);
}
