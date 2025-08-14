import { useState, useRef, useCallback, useEffect } from "react";
import { toast } from "@revlentless/ui/components/sonner";

// TODO: make this a public VITE env
const AUDIO_WORKLET_URL = "/audio-processor.js";

type VoiceRecorderStatus = "idle" | "recording" | "stopping";

export function useVoiceRecorder(onAudioData: (data: ArrayBuffer) => void) {
  const [status, setStatus] = useState<VoiceRecorderStatus>("idle");
  // Use a ref to hold the current status for use in stale closures.
  const statusRef = useRef(status);

  const audioContextRef = useRef<AudioContext | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const workletNodeRef = useRef<AudioWorkletNode | null>(null);

  // Keep the ref synchronized with the state.
  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  const start = useCallback(async () => {
    // Check ref to prevent race conditions.
    if (statusRef.current !== "idle") return;

    setStatus("recording"); // Optimistically set status

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      mediaStreamRef.current = stream;

      const context = new AudioContext();
      audioContextRef.current = context;

      await context.audioWorklet.addModule(AUDIO_WORKLET_URL);
      const source = context.createMediaStreamSource(stream);
      const processor = new AudioWorkletNode(context, "audio-processor");

      processor.port.onmessage = (event) => {
        // Check the ref here. This always has the latest value and is not stale.
        if (statusRef.current === "recording") {
          onAudioData(event.data);
        }
      };

      source.connect(processor);
      workletNodeRef.current = processor;
      toast.info("Microphone is on. Start speaking.");
    } catch (error) {
      console.error("Failed to start voice recorder:", error);
      toast.error("Could not access microphone.");
      setStatus("idle"); // Rollback status on error
    }
  }, [onAudioData]);

  const stop = useCallback(() => {
    if (statusRef.current !== "recording") return;

    setStatus("stopping");
    if (mediaStreamRef.current) {
      mediaStreamRef.current.getTracks().forEach((track) => track.stop());
      mediaStreamRef.current = null;
    }
    if (workletNodeRef.current) {
      workletNodeRef.current.disconnect();
      workletNodeRef.current = null;
    }
    if (audioContextRef.current) {
      // It's better to check the state before closing
      if (audioContextRef.current.state !== "closed") {
        audioContextRef.current.close();
      }
      audioContextRef.current = null;
    }
    setStatus("idle");
    toast.info("Microphone off.");
  }, []);

  const isRecording = status === "recording";

  return { start, stop, isRecording, status };
}
