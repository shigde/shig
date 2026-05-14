# Low-Latency Pipeline: WebRTC RTP → FFmpeg → MoQ

If the client already sends H.264, and FFmpeg only remuxes/transcodes minimally:

### RTP (H.264 + Opus)
→ FFmpeg
→ H.264 (copy) + AAC (encode)
→ fMP4 / CMAF
→ MoQ Publisher

This pipeline can be quite fast.

### Expected Latency

#### With good tuning
```
~300 ms – 1.5 s
```
#### Very well optimized
```
~150 – 500 ms
```
#### Conservative (FFmpeg + AAC + fMP4)
```
~500 ms – 2 s
```
### Where Latency Comes From
```
WebRTC RTP jitter buffer      ~20–100 ms
FFmpeg RTP input              ~50–300 ms
AAC encoding                  ~20–100 ms
fMP4 fragment generation      depends on frame/GOP
MoQ publish/relay             ~10–100 ms
Client buffer/decoding        ~100–500 ms
```
#### The biggest lever is:

+ Fragment duration / keyframe interval (GOP)
Key Insight

#### When using fMP4 with:
```
-movflags frag_keyframe+empty_moov+default_base_moof
```
Fragments are typically cut at keyframes.

If your client sends a keyframe only every 2 seconds, then you effectively get:

```
~2 seconds fragment latency
```

 even if everything else is optimized.

#### For Low Latency: Use Short GOPs

Recommended WebRTC encoder settings:
```
30 fps
Keyframe every 0.5–1 second
```

Examples:
```
GOP 15 @ 30 fps → ~500 ms
GOP 30 @ 30 fps → ~1 s
Fast FFmpeg Pipeline (H.264 Input)
-c:v copy \
-c:a aac \
-f mp4 \
-movflags frag_keyframe+empty_moov+default_base_moof+separate_moof
```

#### Additional Low-Latency Input Flags

```
-fflags nobuffer \
-flags low_delay \
-analyzeduration 0 \
-probesize 32
```

#### Realistic End-to-End Expectation

If your client sends:
```
H.264 @ 30 fps
Keyframe every 0.5–1 s
Opus audio
```

##### You can expect:
```
~0.7 – 1.5 seconds end-to-end latency
```

##### With aggressive tuning
``` 
~300 – 700 ms
``` 


