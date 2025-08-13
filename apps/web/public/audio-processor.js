// This processor takes raw float32 audio data from the microphone,
// converts it to 16-bit PCM, and sends it back to the main thread.
class AudioProcessor extends AudioWorkletProcessor {
  process(inputs, outputs, parameters) {
    // We only care about the first input, and the first channel of that input.
    const input = inputs[0];
    if (!input || input.length === 0) {
      return true; // Keep processor alive
    }

    const channelData = input[0];
    if (!channelData) {
      return true;
    }

    // Convert Float32Array from -1.0 to 1.0 range to Int16Array (-32768 to 32767)
    const pcm16 = new Int16Array(channelData.length);
    for (let i = 0; i < channelData.length; i++) {
      // Clamp the value to be safe
      const s = Math.max(-1, Math.min(1, channelData[i]));
      // Convert to 16-bit integer
      pcm16[i] = s < 0 ? s * 0x8000 : s * 0x7fff;
    }

    // Post the raw buffer back to the main thread.
    // The second argument is a list of transferable objects, making this very efficient.
    this.port.postMessage(pcm16.buffer, [pcm16.buffer]);

    // Return true to indicate the processor should not be terminated.
    return true;
  }
}

registerProcessor("audio-processor", AudioProcessor);
