// The backend sends PCM16 data at a specific sample rate.
// OpenAI uses 24000, which we will standardize on for the frontend player.
const SAMPLE_RATE = 24000;

/**
 * A helper function to decode a base64 string into an ArrayBuffer using browser-native APIs.
 * @param base64 The base64 string to decode.
 * @returns An ArrayBuffer containing the raw binary data.
 */
function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const binaryString = atob(base64); // Decode base64 to a raw binary string
  const len = binaryString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i); // Convert each character to a byte
  }
  return bytes.buffer;
}

export class AudioPlayer {
  private audioContext: AudioContext;
  private audioQueue: AudioBuffer[] = [];
  private isPlaying = false;
  private nextStartTime = 0;

  constructor() {
    // Lazily create AudioContext on first interaction if needed, or create it here.
    // Creating it here is fine for modern browsers.
    this.audioContext = new AudioContext({ sampleRate: SAMPLE_RATE });
  }

  public addChunk = async (base64Chunk: string) => {
    try {
      // 1. Decode base64 using the browser-native function.
      const chunkBuffer = base64ToArrayBuffer(base64Chunk);

      // 2. Create a Float32Array from the 16-bit PCM data.
      const pcm16 = new Int16Array(chunkBuffer);
      const pcm32 = new Float32Array(pcm16.length);
      for (let i = 0; i < pcm16.length; i++) {
        pcm32[i] = pcm16[i] / 32768.0; // Normalize to [-1.0, 1.0]
      }

      // 3. Create an AudioBuffer.
      if (pcm32.length === 0) return;
      const audioBuffer = this.audioContext.createBuffer(
        1, // 1 channel (mono)
        pcm32.length,
        this.audioContext.sampleRate
      );
      audioBuffer.copyToChannel(pcm32, 0);

      // 4. Add to queue and start playback if not already playing.
      this.audioQueue.push(audioBuffer);
      if (!this.isPlaying) {
        this.playQueue();
      }
    } catch (error) {
      console.error("Error processing audio chunk:", error);
    }
  };

  private playQueue = () => {
    if (this.audioQueue.length === 0) {
      this.isPlaying = false;
      return;
    }

    this.isPlaying = true;
    const bufferToPlay = this.audioQueue.shift()!;
    const source = this.audioContext.createBufferSource();
    source.buffer = bufferToPlay;
    source.connect(this.audioContext.destination);

    const currentTime = this.audioContext.currentTime;
    // Schedule playback to start where the last chunk ended, or now if we're behind.
    const startTime = Math.max(currentTime, this.nextStartTime);

    source.start(startTime);

    // Schedule the next chunk to play right after this one ends.
    this.nextStartTime = startTime + bufferToPlay.duration;

    source.onended = this.playQueue;
  };

  public stop() {
    this.audioQueue = [];
    this.isPlaying = false;
    // TODO: stop the current source node if needed,
    // but for now, just clearing the queue is sufficient.
    if (this.audioContext.state === "running") {
      this.audioContext.close();
    }
    // Recreate a new context for the next playback session
    this.audioContext = new AudioContext({ sampleRate: SAMPLE_RATE });
  }
}
