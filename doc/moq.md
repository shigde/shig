

## CMAF Fragment to a MoQ Frame

catalog.json
video track → CMAF fragments
audio track → CMAF fragments


a CMAF Fragment = a MoQ Frame or a MoQ Group

FFmpeg generates both CMAF tracks from the same input
+
CMAF Timestamps remain unchanged
+
Client synchronizes after these timestamps





### CMAF Track Writer

### CMAF Splitter
```
ftyp + moov       → init segment
moof + mdat       → fragment
```
