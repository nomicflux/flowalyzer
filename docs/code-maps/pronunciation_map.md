src/bin/pronunciation.rs
- main
  line: 42
  called_from:
    - none
  calls:
    - config (src/pronunciation/session/runtime.rs:419)
    - default (src/pronunciation/alignment/mod.rs:33)
    - load_clip (src/pronunciation/mod.rs:65)
    - new (src/pronunciation/alignment/mod.rs:8)
    - sample_rate (src/pronunciation/session/engine.rs:63)
    - spawn (src/pronunciation/session/runtime.rs:54)

src/pronunciation/alignment/mod.rs
- new
  line: 8
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - default (src/pronunciation/alignment/mod.rs:33)
    - default (src/pronunciation/features/mod.rs:112)
    - drain_snapshots (src/pronunciation/session/runtime.rs:411)
    - draw_comparison_panel (src/ui/screens/session.rs:296)
    - draw_history_line (src/ui/screens/session.rs:266)
    - draw_history_plot (src/ui/screens/session.rs:244)
    - draw_row (src/ui/screens/session.rs:334)
    - dummy_app (src/ui/screens/session.rs:424)
    - extract_audio_range (src/pronunciation/mod.rs:93)
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
    - main (src/bin/pronunciation.rs:42)
    - new (src/pronunciation/features/mod.rs:41)
    - new (src/pronunciation/mod.rs:28)
    - new (src/pronunciation/session/engine.rs:15)
    - new (src/ui/screens/session.rs:199)
    - new (src/ui/screens/session.rs:24)
    - replay_reference (src/pronunciation/session/runtime.rs:454)
    - run (src/pronunciation/session/runtime.rs:98)
    - shutdown (src/pronunciation/session/runtime.rs:466)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn (src/pronunciation/session/runtime.rs:54)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start (src/pronunciation/session/runtime.rs:442)
    - start_capture (src/pronunciation/session/runtime.rs:178)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
    - stop (src/pronunciation/session/runtime.rs:448)
    - stop_replay (src/pronunciation/session/runtime.rs:460)
    - validate_clip_duration (src/pronunciation/mod.rs:121)
    - validate_time_range (src/pronunciation/mod.rs:133)
  calls:
    - none
- align
  line: 12
  called_from:
    - none
  calls:
    - align_features (src/pronunciation/alignment/mod.rs:38)
    - global_offset_ms (src/pronunciation/session/engine.rs:76)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- default
  line: 33
  called_from:
    - default (src/pronunciation/features/mod.rs:112)
    - default (src/pronunciation/session/config.rs:13)
    - dummy_app (src/ui/screens/session.rs:424)
    - main (src/bin/pronunciation.rs:42)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - update (src/ui/screens/session.rs:175)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
- align_features
  line: 38
  called_from:
    - align (src/pronunciation/alignment/mod.rs:12)
    - process_chunk (src/pronunciation/session/engine.rs:35)
  calls:
    - compute_contour_band (src/pronunciation/alignment/mod.rs:96)
    - compute_energy_error (src/pronunciation/alignment/mod.rs:75)
    - compute_similarity (src/pronunciation/alignment/mod.rs:81)
    - global_offset_ms (src/pronunciation/session/engine.rs:76)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- compute_energy_error
  line: 75
  called_from:
    - align_features (src/pronunciation/alignment/mod.rs:38)
  calls:
    - none
- compute_similarity
  line: 81
  called_from:
    - align_features (src/pronunciation/alignment/mod.rs:38)
  calls:
    - none
- compute_contour_band
  line: 96
  called_from:
    - align_features (src/pronunciation/alignment/mod.rs:38)
  calls:
    - none

src/pronunciation/features/mod.rs
- from_sample_rate
  line: 13
  called_from:
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - new (src/pronunciation/session/engine.rs:15)
    - required_tail_len (src/pronunciation/session/runtime.rs:260)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
  calls:
    - sample_rate (src/pronunciation/session/engine.rs:63)
    - scaled_samples (src/pronunciation/features/mod.rs:117)
- new
  line: 41
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - default (src/pronunciation/alignment/mod.rs:33)
    - default (src/pronunciation/features/mod.rs:112)
    - drain_snapshots (src/pronunciation/session/runtime.rs:411)
    - draw_comparison_panel (src/ui/screens/session.rs:296)
    - draw_history_line (src/ui/screens/session.rs:266)
    - draw_history_plot (src/ui/screens/session.rs:244)
    - draw_row (src/ui/screens/session.rs:334)
    - dummy_app (src/ui/screens/session.rs:424)
    - extract_audio_range (src/pronunciation/mod.rs:93)
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
    - main (src/bin/pronunciation.rs:42)
    - new (src/pronunciation/alignment/mod.rs:8)
    - new (src/pronunciation/mod.rs:28)
    - new (src/pronunciation/session/engine.rs:15)
    - new (src/ui/screens/session.rs:199)
    - new (src/ui/screens/session.rs:24)
    - replay_reference (src/pronunciation/session/runtime.rs:454)
    - run (src/pronunciation/session/runtime.rs:98)
    - shutdown (src/pronunciation/session/runtime.rs:466)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn (src/pronunciation/session/runtime.rs:54)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start (src/pronunciation/session/runtime.rs:442)
    - start_capture (src/pronunciation/session/runtime.rs:178)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
    - stop (src/pronunciation/session/runtime.rs:448)
    - stop_replay (src/pronunciation/session/runtime.rs:460)
    - validate_clip_duration (src/pronunciation/mod.rs:121)
    - validate_time_range (src/pronunciation/mod.rs:133)
  calls:
    - none
- extract_reference
  line: 45
  called_from:
    - create_engine (src/pronunciation/session/runtime.rs:46)
  calls:
    - frame_energy (src/pronunciation/features/mod.rs:121)
    - frame_pitch (src/pronunciation/features/mod.rs:125)
    - new (src/pronunciation/alignment/mod.rs:8)
    - sample_rate (src/pronunciation/session/engine.rs:63)
    - start (src/pronunciation/session/runtime.rs:442)
- extract_chunk
  line: 76
  called_from:
    - process_chunk (src/pronunciation/session/engine.rs:35)
  calls:
    - frame_energy (src/pronunciation/features/mod.rs:121)
    - frame_pitch (src/pronunciation/features/mod.rs:125)
    - new (src/pronunciation/alignment/mod.rs:8)
    - sample_rate (src/pronunciation/session/engine.rs:63)
    - start (src/pronunciation/session/runtime.rs:442)
- default
  line: 112
  called_from:
    - default (src/pronunciation/alignment/mod.rs:33)
    - default (src/pronunciation/session/config.rs:13)
    - dummy_app (src/ui/screens/session.rs:424)
    - main (src/bin/pronunciation.rs:42)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - update (src/ui/screens/session.rs:175)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
- scaled_samples
  line: 117
  called_from:
    - from_sample_rate (src/pronunciation/features/mod.rs:13)
  calls:
    - sample_rate (src/pronunciation/session/engine.rs:63)
- frame_energy
  line: 121
  called_from:
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
  calls:
    - none
- frame_pitch
  line: 125
  called_from:
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
  calls:
    - autocorrelation (src/pronunciation/features/mod.rs:140)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- autocorrelation
  line: 140
  called_from:
    - frame_pitch (src/pronunciation/features/mod.rs:125)
  calls:
    - none

src/pronunciation/mod.rs
- new
  line: 28
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - default (src/pronunciation/alignment/mod.rs:33)
    - default (src/pronunciation/features/mod.rs:112)
    - drain_snapshots (src/pronunciation/session/runtime.rs:411)
    - draw_comparison_panel (src/ui/screens/session.rs:296)
    - draw_history_line (src/ui/screens/session.rs:266)
    - draw_history_plot (src/ui/screens/session.rs:244)
    - draw_row (src/ui/screens/session.rs:334)
    - dummy_app (src/ui/screens/session.rs:424)
    - extract_audio_range (src/pronunciation/mod.rs:93)
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
    - main (src/bin/pronunciation.rs:42)
    - new (src/pronunciation/alignment/mod.rs:8)
    - new (src/pronunciation/features/mod.rs:41)
    - new (src/pronunciation/session/engine.rs:15)
    - new (src/ui/screens/session.rs:199)
    - new (src/ui/screens/session.rs:24)
    - replay_reference (src/pronunciation/session/runtime.rs:454)
    - run (src/pronunciation/session/runtime.rs:98)
    - shutdown (src/pronunciation/session/runtime.rs:466)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn (src/pronunciation/session/runtime.rs:54)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start (src/pronunciation/session/runtime.rs:442)
    - start_capture (src/pronunciation/session/runtime.rs:178)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
    - stop (src/pronunciation/session/runtime.rs:448)
    - stop_replay (src/pronunciation/session/runtime.rs:460)
    - validate_clip_duration (src/pronunciation/mod.rs:121)
    - validate_time_range (src/pronunciation/mod.rs:133)
  calls:
    - none
- fmt
  line: 36
  called_from:
    - none
  calls:
    - sample_rate (src/pronunciation/session/engine.rs:63)
- from_samples
  line: 53
  called_from:
    - clip_from_audio (src/pronunciation/mod.rs:117)
    - dummy_app (src/ui/screens/session.rs:424)
  calls:
    - sample_rate (src/pronunciation/session/engine.rs:63)
- load_clip
  line: 65
  called_from:
    - main (src/bin/pronunciation.rs:42)
  calls:
    - clip_from_audio (src/pronunciation/mod.rs:117)
- apply_recipe_to_range
  line: 71
  called_from:
    - none
  calls:
    - clip_from_audio (src/pronunciation/mod.rs:117)
    - extract_audio_range (src/pronunciation/mod.rs:93)
    - new (src/pronunciation/alignment/mod.rs:8)
    - start (src/pronunciation/session/runtime.rs:442)
    - validate_clip_duration (src/pronunciation/mod.rs:121)
- extract_audio_range
  line: 93
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
    - sample_rate (src/pronunciation/session/engine.rs:63)
    - validate_time_range (src/pronunciation/mod.rs:133)
- clip_from_audio
  line: 117
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
    - load_clip (src/pronunciation/mod.rs:65)
  calls:
    - from_samples (src/pronunciation/mod.rs:53)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- validate_clip_duration
  line: 121
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
- validate_time_range
  line: 133
  called_from:
    - extract_audio_range (src/pronunciation/mod.rs:93)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)

src/pronunciation/session/config.rs
- default
  line: 13
  called_from:
    - default (src/pronunciation/alignment/mod.rs:33)
    - default (src/pronunciation/features/mod.rs:112)
    - dummy_app (src/ui/screens/session.rs:424)
    - main (src/bin/pronunciation.rs:42)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - update (src/ui/screens/session.rs:175)
  calls:
    - sample_rate (src/pronunciation/session/engine.rs:63)

src/pronunciation/session/engine.rs
- new
  line: 15
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - default (src/pronunciation/alignment/mod.rs:33)
    - default (src/pronunciation/features/mod.rs:112)
    - drain_snapshots (src/pronunciation/session/runtime.rs:411)
    - draw_comparison_panel (src/ui/screens/session.rs:296)
    - draw_history_line (src/ui/screens/session.rs:266)
    - draw_history_plot (src/ui/screens/session.rs:244)
    - draw_row (src/ui/screens/session.rs:334)
    - dummy_app (src/ui/screens/session.rs:424)
    - extract_audio_range (src/pronunciation/mod.rs:93)
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
    - main (src/bin/pronunciation.rs:42)
    - new (src/pronunciation/alignment/mod.rs:8)
    - new (src/pronunciation/features/mod.rs:41)
    - new (src/pronunciation/mod.rs:28)
    - new (src/ui/screens/session.rs:199)
    - new (src/ui/screens/session.rs:24)
    - replay_reference (src/pronunciation/session/runtime.rs:454)
    - run (src/pronunciation/session/runtime.rs:98)
    - shutdown (src/pronunciation/session/runtime.rs:466)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn (src/pronunciation/session/runtime.rs:54)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start (src/pronunciation/session/runtime.rs:442)
    - start_capture (src/pronunciation/session/runtime.rs:178)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
    - stop (src/pronunciation/session/runtime.rs:448)
    - stop_replay (src/pronunciation/session/runtime.rs:460)
    - validate_clip_duration (src/pronunciation/mod.rs:121)
    - validate_time_range (src/pronunciation/mod.rs:133)
  calls:
    - from_sample_rate (src/pronunciation/features/mod.rs:13)
    - global_sample_counter (src/pronunciation/session/engine.rs:86)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- seed_tail
  line: 28
  called_from:
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
  calls:
    - global_sample_counter (src/pronunciation/session/engine.rs:86)
    - required_tail_len (src/pronunciation/session/engine.rs:72)
    - start (src/pronunciation/session/runtime.rs:442)
- process_chunk
  line: 35
  called_from:
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
  calls:
    - align_features (src/pronunciation/alignment/mod.rs:38)
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - global_offset_ms (src/pronunciation/session/engine.rs:76)
    - global_sample_counter (src/pronunciation/session/engine.rs:86)
    - required_tail_len (src/pronunciation/session/engine.rs:72)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- sample_rate
  line: 63
  called_from:
    - align (src/pronunciation/alignment/mod.rs:12)
    - align_features (src/pronunciation/alignment/mod.rs:38)
    - clip_from_audio (src/pronunciation/mod.rs:117)
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - default (src/pronunciation/session/config.rs:13)
    - extract_audio_range (src/pronunciation/mod.rs:93)
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
    - fmt (src/pronunciation/mod.rs:36)
    - frame_pitch (src/pronunciation/features/mod.rs:125)
    - from_sample_rate (src/pronunciation/features/mod.rs:13)
    - from_samples (src/pronunciation/mod.rs:53)
    - global_offset_ms (src/pronunciation/session/engine.rs:76)
    - main (src/bin/pronunciation.rs:42)
    - new (src/pronunciation/session/engine.rs:15)
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - process_chunk (src/pronunciation/session/engine.rs:35)
    - required_tail_len (src/pronunciation/session/runtime.rs:260)
    - scaled_samples (src/pronunciation/features/mod.rs:117)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start_capture (src/pronunciation/session/runtime.rs:178)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
  calls:
    - none
- reset
  line: 67
  called_from:
    - run (src/pronunciation/session/runtime.rs:98)
  calls:
    - clear (src/ui/screens/session.rs:219)
    - global_sample_counter (src/pronunciation/session/engine.rs:86)
- required_tail_len
  line: 72
  called_from:
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - process_chunk (src/pronunciation/session/engine.rs:35)
    - required_tail_len (src/pronunciation/session/runtime.rs:260)
    - seed_tail (src/pronunciation/session/engine.rs:28)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
  calls:
    - none
- global_offset_ms
  line: 76
  called_from:
    - align (src/pronunciation/alignment/mod.rs:12)
    - align_features (src/pronunciation/alignment/mod.rs:38)
    - process_chunk (src/pronunciation/session/engine.rs:35)
  calls:
    - global_sample_counter (src/pronunciation/session/engine.rs:86)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- tail_samples
  line: 82
  called_from:
    - none
  calls:
    - none
- global_sample_counter
  line: 86
  called_from:
    - global_offset_ms (src/pronunciation/session/engine.rs:76)
    - new (src/pronunciation/session/engine.rs:15)
    - process_chunk (src/pronunciation/session/engine.rs:35)
    - reset (src/pronunciation/session/engine.rs:67)
    - seed_tail (src/pronunciation/session/engine.rs:28)
  calls:
    - none

src/pronunciation/session/runtime.rs
- create_engine
  line: 46
  called_from:
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
  calls:
    - extract_reference (src/pronunciation/features/mod.rs:45)
    - from_sample_rate (src/pronunciation/features/mod.rs:13)
    - new (src/pronunciation/alignment/mod.rs:8)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- spawn
  line: 54
  called_from:
    - dummy_app (src/ui/screens/session.rs:424)
    - main (src/bin/pronunciation.rs:42)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
  calls:
    - config (src/pronunciation/session/runtime.rs:419)
    - new (src/pronunciation/alignment/mod.rs:8)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
- spawn_with_capture_builder
  line: 61
  called_from:
    - spawn (src/pronunciation/session/runtime.rs:54)
  calls:
    - config (src/pronunciation/session/runtime.rs:419)
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - initial_snapshot (src/pronunciation/session/runtime.rs:423)
    - new (src/pronunciation/alignment/mod.rs:8)
    - reference_playing (src/ui/screens/session.rs:57)
    - run (src/pronunciation/session/runtime.rs:98)
    - sample_rate (src/pronunciation/session/engine.rs:63)
    - spawn (src/pronunciation/session/runtime.rs:54)
- run
  line: 98
  called_from:
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
  calls:
    - clear_buffer (src/pronunciation/session/runtime.rs:321)
    - initial_snapshot (src/pronunciation/session/runtime.rs:423)
    - new (src/pronunciation/alignment/mod.rs:8)
    - poll_playback_completion (src/pronunciation/session/runtime.rs:301)
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - recording (src/ui/screens/session.rs:53)
    - reference_playing (src/ui/screens/session.rs:57)
    - reset (src/pronunciation/session/engine.rs:67)
    - snapshot (src/ui/screens/session.rs:49)
    - snapshot_from_last (src/pronunciation/session/runtime.rs:251)
    - start_capture (src/pronunciation/session/runtime.rs:178)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
    - stop (src/pronunciation/session/runtime.rs:448)
    - stop_reference_playback (src/pronunciation/session/runtime.rs:291)
- start_capture
  line: 178
  called_from:
    - run (src/pronunciation/session/runtime.rs:98)
  calls:
    - config (src/pronunciation/session/runtime.rs:419)
    - new (src/pronunciation/alignment/mod.rs:8)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- process_capture_chunk
  line: 188
  called_from:
    - run (src/pronunciation/session/runtime.rs:98)
  calls:
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - config (src/pronunciation/session/runtime.rs:419)
    - process_chunk (src/pronunciation/session/engine.rs:35)
    - required_tail_len (src/pronunciation/session/engine.rs:72)
    - sample_rate (src/pronunciation/session/engine.rs:63)
    - seed_tail (src/pronunciation/session/engine.rs:28)
    - snapshot (src/ui/screens/session.rs:49)
    - snapshot_for (src/pronunciation/session/runtime.rs:247)
- snapshot_for
  line: 247
  called_from:
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
  calls:
    - recording (src/ui/screens/session.rs:53)
    - snapshot_internal (src/pronunciation/session/runtime.rs:265)
- snapshot_from_last
  line: 251
  called_from:
    - run (src/pronunciation/session/runtime.rs:98)
  calls:
    - recording (src/ui/screens/session.rs:53)
    - snapshot_internal (src/pronunciation/session/runtime.rs:265)
- required_tail_len
  line: 260
  called_from:
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - process_chunk (src/pronunciation/session/engine.rs:35)
    - required_tail_len (src/pronunciation/session/engine.rs:72)
    - seed_tail (src/pronunciation/session/engine.rs:28)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
  calls:
    - config (src/pronunciation/session/runtime.rs:419)
    - from_sample_rate (src/pronunciation/features/mod.rs:13)
    - sample_rate (src/pronunciation/session/engine.rs:63)
- snapshot_internal
  line: 265
  called_from:
    - snapshot_for (src/pronunciation/session/runtime.rs:247)
    - snapshot_from_last (src/pronunciation/session/runtime.rs:251)
  calls:
    - recording (src/ui/screens/session.rs:53)
    - reference_playing (src/ui/screens/session.rs:57)
- start_reference_playback
  line: 273
  called_from:
    - run (src/pronunciation/session/runtime.rs:98)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
    - reference_playing (src/ui/screens/session.rs:57)
    - sample_rate (src/pronunciation/session/engine.rs:63)
    - stop_reference_playback (src/pronunciation/session/runtime.rs:291)
- stop_reference_playback
  line: 291
  called_from:
    - run (src/pronunciation/session/runtime.rs:98)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
  calls:
    - reference_playing (src/ui/screens/session.rs:57)
    - stop (src/pronunciation/session/runtime.rs:448)
- poll_playback_completion
  line: 301
  called_from:
    - run (src/pronunciation/session/runtime.rs:98)
  calls:
    - reference_playing (src/ui/screens/session.rs:57)
- clear_buffer
  line: 321
  called_from:
    - run (src/pronunciation/session/runtime.rs:98)
  calls:
    - clear (src/ui/screens/session.rs:219)
- required_raw_samples
  line: 328
  called_from:
    - buffered_collection_emits_and_retain_remainder (src/pronunciation/session/runtime.rs:527)
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - required_samples_scale_with_input_rate (src/pronunciation/session/runtime.rs:487)
    - shutdown (src/pronunciation/session/runtime.rs:466)
  calls:
    - none
- collect_resampled_chunk
  line: 333
  called_from:
    - resampled_chunk_meets_target_length (src/pronunciation/session/runtime.rs:494)
    - returns_none_when_no_chunks_available (src/pronunciation/session/runtime.rs:507)
    - same_rate_chunks_meet_target_length (src/pronunciation/session/runtime.rs:514)
    - shutdown (src/pronunciation/session/runtime.rs:466)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
    - required_raw_samples (src/pronunciation/session/runtime.rs:328)
- collect_resampled_chunk_from_buffer
  line: 376
  called_from:
    - buffered_collection_emits_and_retain_remainder (src/pronunciation/session/runtime.rs:527)
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - shutdown (src/pronunciation/session/runtime.rs:466)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
  calls:
    - config (src/pronunciation/session/runtime.rs:419)
    - initial_snapshot (src/pronunciation/session/runtime.rs:423)
    - new (src/pronunciation/alignment/mod.rs:8)
    - required_raw_samples (src/pronunciation/session/runtime.rs:328)
- drain_snapshots
  line: 411
  called_from:
    - poll_snapshots (src/ui/screens/session.rs:65)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
    - snapshot (src/ui/screens/session.rs:49)
- config
  line: 419
  called_from:
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - config (src/ui/screens/session.rs:61)
    - dummy_app (src/ui/screens/session.rs:424)
    - main (src/bin/pronunciation.rs:42)
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - required_tail_len (src/pronunciation/session/runtime.rs:260)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn (src/pronunciation/session/runtime.rs:54)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start_capture (src/pronunciation/session/runtime.rs:178)
  calls:
    - none
- initial_snapshot
  line: 423
  called_from:
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - new (src/ui/screens/session.rs:24)
    - run (src/pronunciation/session/runtime.rs:98)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
  calls:
    - none
- join
  line: 427
  called_from:
    - none
  calls:
    - none
- start
  line: 442
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
    - seed_tail (src/pronunciation/session/engine.rs:28)
    - show_top_panel (src/ui/screens/session.rs:71)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
- stop
  line: 448
  called_from:
    - run (src/pronunciation/session/runtime.rs:98)
    - show_top_panel (src/ui/screens/session.rs:71)
    - stop_reference_playback (src/pronunciation/session/runtime.rs:291)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
- replay_reference
  line: 454
  called_from:
    - show_top_panel (src/ui/screens/session.rs:71)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
- stop_replay
  line: 460
  called_from:
    - show_top_panel (src/ui/screens/session.rs:71)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
- shutdown
  line: 466
  called_from:
    - drop (src/ui/screens/session.rs:169)
  calls:
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - new (src/pronunciation/alignment/mod.rs:8)
    - required_raw_samples (src/pronunciation/session/runtime.rs:328)
- required_samples_scale_with_input_rate
  line: 487
  called_from:
    - none
  calls:
    - required_raw_samples (src/pronunciation/session/runtime.rs:328)
- resampled_chunk_meets_target_length
  line: 494
  called_from:
    - none
  calls:
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
- returns_none_when_no_chunks_available
  line: 507
  called_from:
    - none
  calls:
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
- same_rate_chunks_meet_target_length
  line: 514
  called_from:
    - none
  calls:
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
- buffered_collection_emits_and_retain_remainder
  line: 527
  called_from:
    - none
  calls:
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - required_raw_samples (src/pronunciation/session/runtime.rs:328)
- capture_pipeline_produces_voiced_pitch
  line: 549
  called_from:
    - none
  calls:
    - none
- sine_wave
  line: 550
  called_from:
    - none
  calls:
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - config (src/pronunciation/session/runtime.rs:419)
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - default (src/pronunciation/alignment/mod.rs:33)
    - from_sample_rate (src/pronunciation/features/mod.rs:13)
    - new (src/pronunciation/alignment/mod.rs:8)
    - process_chunk (src/pronunciation/session/engine.rs:35)
    - required_tail_len (src/pronunciation/session/engine.rs:72)
    - sample_rate (src/pronunciation/session/engine.rs:63)
    - seed_tail (src/pronunciation/session/engine.rs:28)

src/ui/screens/session.rs
- new
  line: 24
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - default (src/pronunciation/alignment/mod.rs:33)
    - default (src/pronunciation/features/mod.rs:112)
    - drain_snapshots (src/pronunciation/session/runtime.rs:411)
    - draw_comparison_panel (src/ui/screens/session.rs:296)
    - draw_history_line (src/ui/screens/session.rs:266)
    - draw_history_plot (src/ui/screens/session.rs:244)
    - draw_row (src/ui/screens/session.rs:334)
    - dummy_app (src/ui/screens/session.rs:424)
    - extract_audio_range (src/pronunciation/mod.rs:93)
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
    - main (src/bin/pronunciation.rs:42)
    - new (src/pronunciation/alignment/mod.rs:8)
    - new (src/pronunciation/features/mod.rs:41)
    - new (src/pronunciation/mod.rs:28)
    - new (src/pronunciation/session/engine.rs:15)
    - new (src/ui/screens/session.rs:199)
    - replay_reference (src/pronunciation/session/runtime.rs:454)
    - run (src/pronunciation/session/runtime.rs:98)
    - shutdown (src/pronunciation/session/runtime.rs:466)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn (src/pronunciation/session/runtime.rs:54)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start (src/pronunciation/session/runtime.rs:442)
    - start_capture (src/pronunciation/session/runtime.rs:178)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
    - stop (src/pronunciation/session/runtime.rs:448)
    - stop_replay (src/pronunciation/session/runtime.rs:460)
    - validate_clip_duration (src/pronunciation/mod.rs:121)
    - validate_time_range (src/pronunciation/mod.rs:133)
  calls:
    - initial_snapshot (src/pronunciation/session/runtime.rs:423)
    - snapshot (src/ui/screens/session.rs:49)
- apply_snapshot
  line: 36
  called_from:
    - clear_histories_resets_state (src/ui/screens/session.rs:472)
    - histories_clear_on_restart (src/ui/screens/session.rs:482)
    - histories_trim_to_window (src/ui/screens/session.rs:459)
    - poll_snapshots (src/ui/screens/session.rs:65)
  calls:
    - accumulate (src/ui/screens/session.rs:210)
    - clear_histories (src/ui/screens/session.rs:45)
    - recording (src/ui/screens/session.rs:53)
    - snapshot (src/ui/screens/session.rs:49)
- clear_histories
  line: 45
  called_from:
    - apply_snapshot (src/ui/screens/session.rs:36)
    - clear_histories_resets_state (src/ui/screens/session.rs:472)
    - show_top_panel (src/ui/screens/session.rs:71)
  calls:
    - clear (src/ui/screens/session.rs:219)
- snapshot
  line: 49
  called_from:
    - apply_snapshot (src/ui/screens/session.rs:36)
    - clear_histories_resets_state (src/ui/screens/session.rs:472)
    - drain_snapshots (src/pronunciation/session/runtime.rs:411)
    - histories_clear_on_restart (src/ui/screens/session.rs:482)
    - histories_trim_to_window (src/ui/screens/session.rs:459)
    - new (src/ui/screens/session.rs:24)
    - poll_snapshots (src/ui/screens/session.rs:65)
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - recording (src/ui/screens/session.rs:53)
    - reference_playing (src/ui/screens/session.rs:57)
    - run (src/pronunciation/session/runtime.rs:98)
  calls:
    - none
- recording
  line: 53
  called_from:
    - apply_snapshot (src/ui/screens/session.rs:36)
    - run (src/pronunciation/session/runtime.rs:98)
    - show_status (src/ui/screens/session.rs:110)
    - show_top_panel (src/ui/screens/session.rs:71)
    - snapshot_for (src/pronunciation/session/runtime.rs:247)
    - snapshot_from_last (src/pronunciation/session/runtime.rs:251)
    - snapshot_internal (src/pronunciation/session/runtime.rs:265)
    - snapshot_with_alignment (src/ui/screens/session.rs:450)
  calls:
    - snapshot (src/ui/screens/session.rs:49)
- reference_playing
  line: 57
  called_from:
    - poll_playback_completion (src/pronunciation/session/runtime.rs:301)
    - run (src/pronunciation/session/runtime.rs:98)
    - show_status (src/ui/screens/session.rs:110)
    - show_top_panel (src/ui/screens/session.rs:71)
    - snapshot_internal (src/pronunciation/session/runtime.rs:265)
    - snapshot_with_alignment (src/ui/screens/session.rs:450)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
    - stop_reference_playback (src/pronunciation/session/runtime.rs:291)
  calls:
    - snapshot (src/ui/screens/session.rs:49)
- config
  line: 61
  called_from:
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - config (src/pronunciation/session/runtime.rs:419)
    - dummy_app (src/ui/screens/session.rs:424)
    - main (src/bin/pronunciation.rs:42)
    - process_capture_chunk (src/pronunciation/session/runtime.rs:188)
    - required_tail_len (src/pronunciation/session/runtime.rs:260)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn (src/pronunciation/session/runtime.rs:54)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start_capture (src/pronunciation/session/runtime.rs:178)
  calls:
    - none
- poll_snapshots
  line: 65
  called_from:
    - update (src/ui/screens/session.rs:175)
  calls:
    - apply_snapshot (src/ui/screens/session.rs:36)
    - drain_snapshots (src/pronunciation/session/runtime.rs:411)
    - snapshot (src/ui/screens/session.rs:49)
- show_top_panel
  line: 71
  called_from:
    - update (src/ui/screens/session.rs:175)
  calls:
    - clear_histories (src/ui/screens/session.rs:45)
    - recording (src/ui/screens/session.rs:53)
    - reference_playing (src/ui/screens/session.rs:57)
    - replay_reference (src/pronunciation/session/runtime.rs:454)
    - start (src/pronunciation/session/runtime.rs:442)
    - stop (src/pronunciation/session/runtime.rs:448)
    - stop_replay (src/pronunciation/session/runtime.rs:460)
- show_status
  line: 110
  called_from:
    - update (src/ui/screens/session.rs:175)
  calls:
    - recording (src/ui/screens/session.rs:53)
    - reference_playing (src/ui/screens/session.rs:57)
- show_visualizations
  line: 139
  called_from:
    - update (src/ui/screens/session.rs:175)
  calls:
    - draw_comparison_panel (src/ui/screens/session.rs:296)
    - draw_history_plot (src/ui/screens/session.rs:244)
- drop
  line: 169
  called_from:
    - none
  calls:
    - shutdown (src/pronunciation/session/runtime.rs:466)
- update
  line: 175
  called_from:
    - none
  calls:
    - default (src/pronunciation/alignment/mod.rs:33)
    - poll_snapshots (src/ui/screens/session.rs:65)
    - show_status (src/ui/screens/session.rs:110)
    - show_top_panel (src/ui/screens/session.rs:71)
    - show_visualizations (src/ui/screens/session.rs:139)
- new
  line: 199
  called_from:
    - apply_recipe_to_range (src/pronunciation/mod.rs:71)
    - collect_resampled_chunk (src/pronunciation/session/runtime.rs:333)
    - collect_resampled_chunk_from_buffer (src/pronunciation/session/runtime.rs:376)
    - create_engine (src/pronunciation/session/runtime.rs:46)
    - default (src/pronunciation/alignment/mod.rs:33)
    - default (src/pronunciation/features/mod.rs:112)
    - drain_snapshots (src/pronunciation/session/runtime.rs:411)
    - draw_comparison_panel (src/ui/screens/session.rs:296)
    - draw_history_line (src/ui/screens/session.rs:266)
    - draw_history_plot (src/ui/screens/session.rs:244)
    - draw_row (src/ui/screens/session.rs:334)
    - dummy_app (src/ui/screens/session.rs:424)
    - extract_audio_range (src/pronunciation/mod.rs:93)
    - extract_chunk (src/pronunciation/features/mod.rs:76)
    - extract_reference (src/pronunciation/features/mod.rs:45)
    - main (src/bin/pronunciation.rs:42)
    - new (src/pronunciation/alignment/mod.rs:8)
    - new (src/pronunciation/features/mod.rs:41)
    - new (src/pronunciation/mod.rs:28)
    - new (src/pronunciation/session/engine.rs:15)
    - new (src/ui/screens/session.rs:24)
    - replay_reference (src/pronunciation/session/runtime.rs:454)
    - run (src/pronunciation/session/runtime.rs:98)
    - shutdown (src/pronunciation/session/runtime.rs:466)
    - sine_wave (src/pronunciation/session/runtime.rs:550)
    - spawn (src/pronunciation/session/runtime.rs:54)
    - spawn_with_capture_builder (src/pronunciation/session/runtime.rs:61)
    - start (src/pronunciation/session/runtime.rs:442)
    - start_capture (src/pronunciation/session/runtime.rs:178)
    - start_reference_playback (src/pronunciation/session/runtime.rs:273)
    - stop (src/pronunciation/session/runtime.rs:448)
    - stop_replay (src/pronunciation/session/runtime.rs:460)
    - validate_clip_duration (src/pronunciation/mod.rs:121)
    - validate_time_range (src/pronunciation/mod.rs:133)
  calls:
    - none
- accumulate
  line: 210
  called_from:
    - apply_snapshot (src/ui/screens/session.rs:36)
  calls:
    - append_chunk (src/ui/screens/session.rs:229)
- clear
  line: 219
  called_from:
    - clear_buffer (src/pronunciation/session/runtime.rs:321)
    - clear_histories (src/ui/screens/session.rs:45)
    - reset (src/pronunciation/session/engine.rs:67)
  calls:
    - none
- append_chunk
  line: 229
  called_from:
    - accumulate (src/ui/screens/session.rs:210)
  calls:
    - trim_to_window (src/ui/screens/session.rs:237)
- trim_to_window
  line: 237
  called_from:
    - append_chunk (src/ui/screens/session.rs:229)
  calls:
    - none
- draw_history_plot
  line: 244
  called_from:
    - show_visualizations (src/ui/screens/session.rs:139)
  calls:
    - draw_history_line (src/ui/screens/session.rs:266)
    - new (src/pronunciation/alignment/mod.rs:8)
- draw_history_line
  line: 266
  called_from:
    - draw_history_plot (src/ui/screens/session.rs:244)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
- draw_comparison_panel
  line: 296
  called_from:
    - show_visualizations (src/ui/screens/session.rs:139)
  calls:
    - contour_color (src/ui/screens/session.rs:384)
    - draw_row (src/ui/screens/session.rs:334)
    - new (src/pronunciation/alignment/mod.rs:8)
    - similarity_color (src/ui/screens/session.rs:373)
- draw_row
  line: 334
  called_from:
    - draw_comparison_panel (src/ui/screens/session.rs:296)
  calls:
    - new (src/pronunciation/alignment/mod.rs:8)
- similarity_color
  line: 373
  called_from:
    - draw_comparison_panel (src/ui/screens/session.rs:296)
  calls:
    - gradient_color (src/ui/screens/session.rs:395)
- contour_color
  line: 384
  called_from:
    - draw_comparison_panel (src/ui/screens/session.rs:296)
  calls:
    - gradient_color (src/ui/screens/session.rs:395)
- gradient_color
  line: 395
  called_from:
    - contour_color (src/ui/screens/session.rs:384)
    - similarity_color (src/ui/screens/session.rs:373)
  calls:
    - lerp_color (src/ui/screens/session.rs:411)
- lerp_color
  line: 411
  called_from:
    - gradient_color (src/ui/screens/session.rs:395)
  calls:
    - none
- dummy_app
  line: 424
  called_from:
    - clear_histories_resets_state (src/ui/screens/session.rs:472)
    - histories_clear_on_restart (src/ui/screens/session.rs:482)
    - histories_trim_to_window (src/ui/screens/session.rs:459)
  calls:
    - config (src/pronunciation/session/runtime.rs:419)
    - default (src/pronunciation/alignment/mod.rs:33)
    - from_samples (src/pronunciation/mod.rs:53)
    - new (src/pronunciation/alignment/mod.rs:8)
    - spawn (src/pronunciation/session/runtime.rs:54)
- report_with_value
  line: 431
  called_from:
    - clear_histories_resets_state (src/ui/screens/session.rs:472)
    - histories_clear_on_restart (src/ui/screens/session.rs:482)
    - histories_trim_to_window (src/ui/screens/session.rs:459)
  calls:
    - none
- snapshot_with_alignment
  line: 450
  called_from:
    - clear_histories_resets_state (src/ui/screens/session.rs:472)
    - histories_clear_on_restart (src/ui/screens/session.rs:482)
    - histories_trim_to_window (src/ui/screens/session.rs:459)
  calls:
    - recording (src/ui/screens/session.rs:53)
    - reference_playing (src/ui/screens/session.rs:57)
- histories_trim_to_window
  line: 459
  called_from:
    - none
  calls:
    - apply_snapshot (src/ui/screens/session.rs:36)
    - dummy_app (src/ui/screens/session.rs:424)
    - report_with_value (src/ui/screens/session.rs:431)
    - snapshot (src/ui/screens/session.rs:49)
    - snapshot_with_alignment (src/ui/screens/session.rs:450)
- clear_histories_resets_state
  line: 472
  called_from:
    - none
  calls:
    - apply_snapshot (src/ui/screens/session.rs:36)
    - clear_histories (src/ui/screens/session.rs:45)
    - dummy_app (src/ui/screens/session.rs:424)
    - report_with_value (src/ui/screens/session.rs:431)
    - snapshot (src/ui/screens/session.rs:49)
    - snapshot_with_alignment (src/ui/screens/session.rs:450)
- histories_clear_on_restart
  line: 482
  called_from:
    - none
  calls:
    - apply_snapshot (src/ui/screens/session.rs:36)
    - dummy_app (src/ui/screens/session.rs:424)
    - report_with_value (src/ui/screens/session.rs:431)
    - snapshot (src/ui/screens/session.rs:49)
    - snapshot_with_alignment (src/ui/screens/session.rs:450)
