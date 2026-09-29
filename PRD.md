# TuxBuddy — PRD & Status

*Ditulis awalnya di akhir sesi kerja 2026-09-29 (sesi 1) karena network bermasalah. Diupdate besar-besaran di akhir sesi 2 (2026-09-29, sore/malam). **Diupdate lagi di sesi 3 (2026-09-29, malam)** — roaming state machine (gap terbesar §4.4 lama) sekarang jalan, plus SATU BUG SERIUS ke-3 ditemukan+diperbaiki (lihat §4.1 poin 9, soal `PreToolUse` nge-block SEMUA tool call, bukan cuma yang butuh approval). Baca §4 dan §8 dulu sebelum lanjut.*

## 1. Apa itu TuxBuddy

Companion desktop untuk Linux, terinspirasi dari **Coucou** (macOS, lihat konteks di §7), yang menampilkan maskot **Tux** yang bereaksi terhadap aktivitas sesi **Claude Code** — dan sekarang berkembang jadi konsep **desktop pet yang jalan-jalan bebas di layar**, bukan bar yang tersembunyi seperti Coucou.

## 2. Visi hasil akhir (target)

### 2.1 Perilaku maskot
- **Idle**: Tux jalan santai (roam) di sepanjang tepi bawah layar, bisa nyebrang ke monitor lain (multi-monitor diizinkan). *(animasi berdiri/nengok sudah ada — lihat §4; roaming lintas-layar beneran BELUM ada, lihat §4.4)*
- **Butuh perhatian user** (approval/notifikasi/question dari Claude Code): Tux **terbang** ke tengah layar pakai animasi terbang, **berhenti total**, baru menampilkan UI (tombol Allow/Always/Deny, atau kotak jawaban pertanyaan). Urutan ini penting: stop dulu, baru UI muncul — bukan bersamaan. **✅ SUDAH JALAN** (untuk urutan animasi terbang→landing→UI; belum ada gerakan window fisik ke tengah layar, lihat §4.4).
- State visual mengikuti `SessionState`: `Idle, Working, Thinking, Searching, Approval, Question, Error, Finished, RateLimit` — masing-masing punya warna glow, badge ikon, dan **animasi frame-sequence 24fps dari video asli user** (bukan static image lagi). **✅ SELESAI**, lihat §4.1 dan §5.
- Always-on-top secara alami membuat Tux "nembus" di atas aplikasi lain kapan pun dia butuh perhatian — tidak perlu mekanisme khusus, ini konsekuensi dari sifat window always-on-top. **✅ Terverifikasi jalan** via fix GDK_BACKEND=x11, lihat §4.1.

### 2.2 Kontrol interaksi user
- **Drag**: user bisa klik-tahan-geser Tux ke posisi manapun. **BELUM ada.**
- **Klik → menu**: memunculkan menu berisi — lihat notifikasi/riwayat, opsi pause/diemin (stop roaming sementara), info "AI berbasis Claude", dan opsi-opsi lain yang disepakati selama development (belum final list-nya). **Sebagian ✅**: klik kiri (atau kanan) sekarang memunculkan dropdown menu beneran, tapi isinya baru "Install hooks" / "Uninstall hooks" (lihat §4.1). Item lain (riwayat notifikasi, pause, info AI) masih belum ada.
- **Mode kontrol manual (WASD)**: **opt-in only** — hanya aktif kalau user secara eksplisit menyalakan lewat toggle di menu. Tidak pernah aktif default (supaya tidak bentrok dengan mengetik di aplikasi lain). **BELUM ada.**
- Klik tombol approve/deny mengirim keputusan balik ke Claude Code lewat socket, tanpa user perlu alt-tab ke terminal. **✅ SELESAI dan terverifikasi LIVE** terhadap sesi Claude Code asli (lihat §4.1 — temuan besar sesi 2).
- Klik (dobel klik?) juga bisa best-effort jump ke jendela terminal yang memicu event (X11 only, lihat keterbatasan §6). *(kode `terminal_focus.rs` ada, dobel-klik sudah wired di `main.ts`, tapi belum pernah dites end-to-end)*

### 2.3 Integrasi Claude Code
- Install dirinya ke `~/.claude/settings.json` (menambah hook untuk semua event yang relevan), dengan **backup otomatis + preview diff + konfirmasi user** sebelum menulis. **✅ Backend SELESAI dari sesi 1, UI-nya SEKARANG SUDAH TERSAMBUNG (sesi 2)** — dan ternyata **sudah pernah benar-benar terpasang** ke `~/.claude/settings.json` user yang asli (entah dari sesi 1 atau tes manual user), itulah yang memungkinkan penemuan besar di §4.1.
- Semua komunikasi lokal lewat **Unix socket** — tidak ada cloud, tidak ada telemetry.

## 3. Arsitektur (yang sudah dibangun)

```
tuxbuddy/                       (Cargo workspace, npm workspace utk frontend)
├── Cargo.toml                  workspace: src-tauri, crates/protocol, crates/hook-cli
├── crates/
│   ├── protocol/src/lib.rs     HookEnvelope, DecisionMessage, TerminalMeta,
│   │                           is_blocking() (FIXED sesi 2, lihat §4.1),
│   │                           socket_path(), BLOCKING_TIMEOUT_SECS
│   └── hook-cli/src/main.rs    binary `tuxbuddy-hook` — dipanggil Claude Code sbg hook command
├── src-tauri/src/
│   ├── lib.rs                  bootstrap: manage AppState, register commands, spawn socket, window setup
│   ├── app_state.rs            AppState { session: Mutex<SessionState>, pending: Mutex<HashMap<Uuid,oneshot::Sender>> }
│   ├── session.rs              enum SessionState (9 varian, match persis dgn TS)
│   ├── commands.rs             respond_to_event, get_session_state, focus_terminal,
│   │                           preview_hook_diff, install_hooks, uninstall_hooks
│   ├── socket/server.rs        UnixListener accept loop, spawn task per koneksi
│   ├── socket/router.rs        parse envelope → update state → emit ke frontend →
│   │                           (kalau blocking) tunggu respons dgn timeout
│   ├── hooks/installer.rs      baca/backup/merge/preview/write ~/.claude/settings.json
│   ├── hooks/schema.rs         daftar HOOK_EVENTS + timeout per event
│   ├── terminal_focus.rs       best-effort wmctrl by PID (X11 only) — masih belum dites
│   ├── roaming.rs              BARU (sesi 3) — background tokio task: jalan di tepi bawah
│   │                           monitor (lintas monitor), fly-to-center + pause/resume via
│   │                           mpsc RoamCommand, lihat §4.1 poin 8
│   ├── main.rs                 sekarang auto-set GDK_BACKEND=x11 di Wayland (FIX sesi 2)
│   └── window/{mod,x11,layer_shell}.rs   deteksi Wayland/X11, positioning
├── src/                        frontend Vite + TypeScript
│   ├── main.ts                 bootstrap render loop, wiring klik→menu, fly-in→panel, dev-state buttons
│   ├── engine/{easing,tween,particles}.ts
│   ├── tux/{stateConfig,renderTux}.ts    animated frame-sequence renderer (lihat §4.1/§5)
│   ├── ui/decisionPanel.ts     BARU (sesi 2) — Allow/Always/Deny + kotak jawaban Question
│   ├── ui/hooksPanel.ts        BARU (sesi 2) — menu klik: preview+install/uninstall hooks
│   ├── ipc/tauriBridge.ts      listen/invoke wrapper + eventNameToState (provisional!) +
│   │                           previewHookDiff/installHooks/uninstallHooks (BARU sesi 2)
│   └── assets/
│       ├── tux.png             sprite statis lama (sudah TIDAK dipakai kode, dibiarkan sbg referensi)
│       └── states/<name>/f000.png..   BARU (sesi 2) — sequence 24fps per state, lihat §5
├── design/prototype/           prototipe HTML standalone (riwayat eksperimen, lihat §4.2 versi lama)
└── scripts/dev-dump-hook.sh    tool buat schema discovery (belum pernah dipakai — tapi lihat §4.1,
                                 sudah ada cara lain yang kebetulan menemukan info schema real)
```

**Protokol socket**: newline-delimited JSON di `$XDG_RUNTIME_DIR/tuxbuddy/tuxbuddy.sock`.

## 4. Status implementasi — jujur per bagian

### 4.1 SELESAI dan TERVERIFIKASI sesi 2 (2026-09-29) — perubahan besar

**Semua poin di bawah ini sudah dites visual (screenshot beneran, browser DAN native window) dan/atau live terhadap Claude Code asli — bukan cuma "sudah ditulis".**

1. **Temuan besar #1 — `GDK_BACKEND=x11` TERBUKTI menyelesaikan masalah Wayland positioning.** Dites eksplisit: set env var itu bikin `window/mod.rs` masuk jalur X11 (`x11.rs`), dan `set_always_on_top(true)` + `set_position()` Tauri **beneran berfungsi** — diverifikasi pakai `wmctrl`/`xdotool` dari luar app (window kelihatan sebagai window X11 asli lewat XWayland, bisa dipindah, `_NET_WM_STATE_ABOVE`/`STAYS_ON_TOP` beneran ke-set). **Sudah dijadikan default**: `main.rs` sekarang otomatis `set_var("GDK_BACKEND","x11")` kalau mendeteksi sesi Wayland dan user belum override sendiri — user tidak perlu lakukan apa-apa lagi, window akan otomatis always-on-top + bisa diposisikan di laptop KDE Wayland harian.
2. **Temuan besar #2 — Claude bisa screenshot window native di environment ini, kontra klaim PRD lama.** `import -window <id>` (ImageMagick) berhasil capture window Tauri asli setelah dipaksa lewat XWayland (karena sekarang window itu benar-benar window X11). Klaim lama "Claude tidak bisa screenshot window native" **SALAH** — itu cuma benar sebelum fix GDK_BACKEND. Sesi berikutnya: pakai `import -window $(wmctrl -l -G | grep TuxBuddy | awk '{print $1}')` buat verifikasi visual asli, tidak perlu lagi cuma mengandalkan browser dev-server sebagai proxy.
3. **UI Allow/Always/Deny + kotak jawaban Question — SELESAI.** `src/ui/decisionPanel.ts` baru: didorong oleh `envelope.blocking` (bukan `event_name`, supaya robust terhadap schema yang belum pasti). Approval-style → 3 tombol; Question-style (dideteksi dari `payload.tool_name === "AskUserQuestion"` atau ada `payload.question`) → input teks + tombol Kirim. **Diverifikasi visual** via Playwright (browser) DAN dikonfirmasi bekerja LIVE lewat native app (lihat poin 6 di bawah).
4. **Animasi per-state 24fps — SELESAI, ganti total dari sprite statis.** Video asli user (`tux.mp4`, 240 frame @ 24fps, 8 scene: Thinking/Working/Searching/"Finish Working"/Error/Question/Approval/Notification — ternyata **ada label teks ke-bakar di videonya sendiri**, jadi batas antar-scene bisa dibaca persis, bukan tebakan lagi) + `idle-run.mp4` (dipakai porsi berdiri/nengoknya, frame 0-86, untuk state Idle) semuanya diekstrak penuh, di-background-remove pakai **rembg/u2net** (model 176MB akhirnya selesai didownload sesi ini), di-crop ke union-bounding-box per state (supaya tidak jitter), lalu di-render di `renderTux.ts` sebagai sequence frame yang di-ping-pong loop di 24fps asli. Hasil: **Idle, Thinking, Working, Searching, Finished, Error, Question, Approval semuanya animasi asli dari video, bukan pose statis lagi.** RateLimit tetap reuse sequence Idle (tidak ada scene RateLimit di video manapun) + tint oranye dari `stateConfig.ts` — sudah dicek ulang render-nya normal (lihat catatan user di bawah).
5. **Animasi terbang (fly-in) — SELESAI.** User kasih `terbang.mp4` di tengah sesi 2. Diproses sama seperti di atas: frame 30-89 (takeoff→glide→landing, 60 frame) di-rembg, jadi sequence `flying`. `TuxRenderer.playFlyIn(onDone)` play sequence ini SEKALI (bukan ping-pong) lalu panggil callback. `main.ts` sekarang: kalau `envelope.blocking`, panggil `playFlyIn(() => decisionPanel.show(envelope))` — **urutan terbang→landing→BARU UI muncul, sesuai §2.1, sudah diverifikasi visual (5 screenshot berurutan lewat Playwright, lihat progres: terbang→glide→landing→panel muncul TEPAT setelah landing).** Catatan: ini baru animasi visual di tempat (window tidak benar-benar pindah posisi ke tengah layar) — gerakan window fisik masih bagian dari roaming state machine yang belum ada, lihat §4.4.
6. **Temuan besar #3 — LIVE INTEGRATION TEST TIDAK DISENGAJA, dan bug serius ditemukan+diperbaiki.** Saat testing native window, ketahuan bahwa `~/.claude/settings.json` **sudah beneran ter-install** hooks TuxBuddy (dari sesi sebelumnya atau tes manual user) — dan karena app lagi jalan, event hook ASLI dari sesi Claude Code INI SENDIRI (percakapan yang sedang berjalan ini) beneran nyambung ke app dan memicu panel Allow/Deny sungguhan dengan `event_name: "Stop"`! Ini konfirmasi end-to-end pipeline (hook asli → socket → app → UI → klik Allow → balik ke Claude Code) **bekerja secara nyata**, bukan cuma simulasi.
   - **TAPI ini juga membongkar bug serius**: `protocol::is_blocking()` sebelumnya include `"Stop"` dan `"UserPromptSubmit"` sebagai blocking. Karena `tuxbuddy-hook` betul-betul nunggu sampai 115 detik untuk keputusan GUI kalau blocking, ini berarti **SETIAP giliran Claude Code selesai (`Stop`) dan SETIAP prompt user dikirim (`UserPromptSubmit`) bisa hang sampai 115 detik** kalau TuxBuddy lagi jalan dan tidak ada yang klik panel-nya — sangat merusak untuk pemakaian normal (app dimaksudkan jadi companion pasif, bukan yang harus terus dipelototin tiap giliran).
   - **SUDAH DIPERBAIKI**: `is_blocking()` sekarang cuma true untuk `"PreToolUse"` (gerbang permission tool yang asli di Claude Code) dan `"PermissionRequest"` (dipertahankan defensif, kemungkinan besar bukan nama event asli — belum pernah kelihatan). `Stop`/`UserPromptSubmit` tetap di-forward ke frontend (non-blocking) supaya mascot tetap bereaksi visual, tapi TIDAK menahan proses Claude Code lagi. Rebuild sudah jalan otomatis lewat file-watcher `tauri dev`.
   - **Implikasi penting**: kalau ada laporan "Claude Code kerasa lambat/hang" di sesi manapun sebelum fix ini ter-deploy ke instance yang jalan, ini kemungkinan besar penyebabnya.
7. **Menu klik — sebagian selesai.** `src/ui/hooksPanel.ts` baru: klik (kiri, atau kanan sbg alias) di Tux memunculkan dropdown dengan "Install hooks ke Claude Code…" (preview diff dulu, baru konfirmasi tulis) dan "Uninstall hooks". **Diverifikasi visual** browser DAN native (native: klik kanan di window asli via `xdotool`, screenshot via `import`, panel muncul dengan benar; klik tombol Install memicu invoke Tauri sungguhan — sempat nampilkan panel Approval sungguhan karena keburu ada event `Stop` nyata masuk, itulah cara bug #6 ketemu). Item menu lain (riwayat notifikasi, pause, info AI) masih belum ada — scope-nya sengaja dibatasi ke hook-install dulu sesuai prioritas §8 lama.
8. Workspace Rust compile bersih + **3 unit test installer masih lolos** (`cargo test --workspace`) + `tsc --noEmit` bersih, setelah semua perubahan di atas.

### 4.1b SELESAI dan TERVERIFIKASI sesi 3 (2026-09-29, malam) — roaming + bug approval-spam

9. **Roaming state machine — SELESAI (gap terbesar dari §4.4 lama).** `src-tauri/src/roaming.rs` baru: background tokio task yang menggerakkan window beneran (bukan cuma sprite) di sepanjang tepi bawah monitor via `set_position()` tiap ~33ms, bounce di ujung kiri/kanan, dan **lintas monitor** (menghitung union horizontal dari semua `available_monitors()`, pilih monitor mana yang "di bawah" window saat ini untuk y-nya). State dikontrol lewat `RoamCommand` (`Pause`/`Resume`/`FlyToCenter`) via `mpsc` channel, di-manage sebagai `RoamHandle` Tauri state. **Fly-to-center sekarang MEMINDAHKAN WINDOW FISIK** ke tengah monitor (bukan cuma animasi sprite di tempat seperti sebelumnya) — dipicu otomatis dari `socket/router.rs` begitu `envelope.blocking == true` (lihat catatan poin 9 di bawah soal kapan ini sekarang benar-benar terjadi), dengan durasi easing 2.5 detik biar nyambung sama animasi terbang 60-frame yang sudah ada. Roaming otomatis resume setelah keputusan (atau timeout) — window TIDAK PERNAH macet permanen di tengah layar. Juga ditambahkan command `pause_roaming`/`resume_roaming` yang dipanggil frontend saat `hooksPanel` (menu klik) dibuka/ditutup, supaya Tux tidak kabur dari bawah menu yang lagi dibuka.
   - **Diverifikasi end-to-end LIVE** (bukan cuma dibaca kodenya): window start di kiri-bawah monitor, bergerak stabil ~46px/detik (dicek lewat `xdotool getwindowgeometry` berulang), terbang ke tengah PERSIS (750,310 untuk layar 1920×1080 + window 420×460 — exact center) begitu envelope blocking dikirim manual lewat socket, decision panel muncul di atasnya, klik Allow (lewat `xdotool`, perlu `wmctrl -a` + `windowactivate --sync` dulu karena klik sintetis pertama sering tidak konsisten — sama seperti temuan §6 lama) meresolve dan window balik roam dari posisi tengah. Pause saat menu dibuka juga dicek: window diam total selama beberapa detik saat `hooksPanel` terbuka, lalu lanjut jalan lagi setelah ditutup.
   - Keputusan desain: roaming jalan terus di SEMUA state non-attention (bukan cuma `Idle`) — supaya peliharaan desktop ini terasa hidup terus, bukan cuma bergerak saat benar-benar nganggur. Ini interpretasi PRD §2.1 yang belum eksplisit disebut, silakan dikoreksi user kalau maunya beda (misal: cuma roam saat Idle, diam di tempat saat Working/Thinking dst).
   - Posisi awal (`window/x11.rs`) diubah dari top-center jadi bottom-left, jadi titik mulai yang konsisten sebelum roaming task ambil alih.
   - **Belum selesai** dari roaming: animasi jalan/lari (masih pakai pose Idle berdiri yang "meluncur", karena frame lari `idle-run.mp4` 87-239 belum diproses — lihat §4.4 poin 7 lama, masih berlaku), drag-to-reposition (harus bisa interupsi roaming, belum ada), dan belum ada opsi "diemin" permanen di menu (baru pause otomatis pas menu kebuka).

10. **BUG SERIUS #3 ditemukan + diperbaiki — `PreToolUse` ternyata nge-fire untuk SEMUA tool call, bukan cuma yang butuh approval.** Ketemu lewat dogfooding tidak sengaja: karena TuxBuddy sedang jalan (dev mode) SEKALIGUS ada sesi Claude Code asli sedang mengerjakan TuxBuddy sendiri (sesi ini), dan hooks `PreToolUse` sesi 2 sempat diset blocking dengan matcher `"*"` (cocok ke SEMUA tool, lihat `hooks/schema.rs` — tidak ada scoping per-tool), setiap Bash/Edit/Write yang dijalankan Claude Code di sesi manapun di komputer ini memicu window TERBANG KE TENGAH + panel "Allow ...?" beneran di layar user — padahal tidak ada permission sungguhan yang perlu dinegosiasikan, itu cuma tool call biasa. User melaporkan ini langsung ("kenapa dia minta mulu approval padahal claude ngga minta approve apapun").
    - **Root cause**: `PreToolUse` di Claude Code memang fire untuk *setiap* pemanggilan tool, bukan cuma yang butuh izin — asumsi sesi 2 (PRD versi lama) yang menyamakan "event `PreToolUse` masuk" dengan "butuh keputusan manusia" SALAH. Belum ada field di payload hook yang diketahui bisa membedakan "ini beneran butuh approval" vs "tool call rutin".
    - **SUDAH DIPERBAIKI (sementara, defensif)**: `protocol::is_blocking()` sekarang **selalu return `false`** — tidak ada event apapun yang blocking lagi. Semua event tetap di-forward ke frontend (non-blocking) jadi mascot tetap bereaksi visual per state, tapi TIDAK ADA lagi window yang tiba-tiba nge-block/terbang minta approval. **Efek samping: fitur inti Allow/Deny/Question (§2.2) untuk sementara TIDAK PERNAH terpicu dari event asli** sampai ada cara yang lebih pintar buat membedakan "tool call rutin" dari "keputusan yang beneran perlu manusia" — ini keputusan sadar demi keamanan/tidak mengganggu dulu, BUKAN solusi final. UI Allow/Deny/Question sendiri masih berfungsi penuh kalau dipicu manual (sudah dites ulang di sesi 3, lihat poin 9 di atas).
    - **PENTING buat sesi berikutnya**: binary `tuxbuddy-hook` (`crates/hook-cli`) itu **binary terpisah** dari app Tauri utama — `tauri dev`/file-watcher HANYA rebuild app utamanya (`target/debug/tuxbuddy`), BUKAN `target/debug/tuxbuddy-hook` yang dipanggil langsung oleh `~/.claude/settings.json`. Kalau ubah apapun di `crates/protocol` atau `crates/hook-cli`, **WAJIB** `cargo build --bin tuxbuddy-hook` manual juga, atau hook yang beneran dipanggil Claude Code masih pakai binary lama. Ini yang bikin bug di atas sempat kelihatan "belum fix" padahal kodenya sudah benar — proses lama masih terpasang di `~/.claude/settings.json` menunjuk ke binary basi.
    - **SUDAH DIPUTUSKAN user (sesi 3, langsung ditanya)**: user pakai Claude Code dengan **auto-approval/bypass-permissions aktif** di setup-nya sehari-hari — jadi dalam workflow normal dia, nyaris tidak pernah ada momen permission sungguhan (satu-satunya kandidat semacam `sudo`, dan Claude sendiri biasanya menolak/berhenti duluan untuk itu). Kesimpulan user: **fitur Allow/Deny boleh tetap ada di kode "sesuai konteksnya"**, tapi TIDAK perlu buru-buru dibikin canggih/scoping rumit — kondisi `is_blocking()` selalu-`false` sekarang **sudah cukup dan sesuai** untuk workflow dia, bukan cuma tambalan sementara yang urgent dibongkar lagi. Kalau nanti user pakai mode permission yang lebih ketat (bukan auto-approve), baru relevan digali ulang cara scoping-nya (opsi b/c yang tadinya diusulkan: scoping matcher ke tool tertentu, atau cari sinyal lain di payload). **Prioritas untuk ini diturunkan** di §8 — bukan lagi item #1.

11. **Ukuran window diperkecil sesuai feedback user** — dari 420×460 jadi **240×270**. Kanvas tetap digambar di resolusi internal 420×420 (biar sprite/particle/badge tidak perlu ditulis ulang skalanya), lalu di-downscale ke 220×220 lewat CSS (`src/styles.css`) — jadi satu baris CSS, bukan crop. Sempat kelihatan "crop" di satu percobaan karena `tauri.conf.json` (ukuran window) butuh **restart penuh proses**, bukan cuma hot-reload Vite — sesi berikutnya kalau ubah ukuran window lagi, ingat untuk kill+restart `npm run tauri dev`, bukan cuma edit file.

### 4.2 Riwayat eksperimen visual (sesi 1, di `design/prototype/tux-prototype.html`, standalone — BUKAN bagian app asli)
1. **v1**: gambar Tux 100% prosedural (Canvas API) — ditolak, jauh dari referensi.
2. **v2**: masih prosedural tapi diperbaiki — masih ditolak.
3. **v3 (pivot besar)**: pakai `~/Downloads/tux.png` langsung sebagai sprite, dianimasikan whole-body. **Disetujui user**, jadi basis awal (lalu di sesi 2 diganti total jadi frame-sequence dari video, lihat §4.1 poin 4).
4. Overlay mata terpisah — dicoba, gagal presisi, **dihapus total atas permintaan user**.

### 4.3 Catatan gaya kerja dari user (sesi 2)
- User menegaskan: animasi per-state itu **representasi kondisi asli sesi Claude Code** (lewat hook event beneran), BUKAN sesuatu yang di-klik manual satu-satu oleh siapapun. Tombol dev-state (`#dev-states` di `index.html`/`main.ts`) memang cuma alat bantu testing — sudah dan tetap ditandai dev-only di kode, tidak berubah.
- User eksplisit: kalau Tux diklik, harus muncul menu dropdown (bukan cuma animasi kecil/squish) — makanya klik sekarang membuka `hooksPanel`.
- User sempat lapor "animasi RateLimit salah" di satu titik sesi — sudah di-re-verifikasi setelah semua batch selesai dan terlihat render dengan benar (sprite Idle + tint oranye + partikel zzz/sweat). Kemungkinan besar laporan itu soal state transien SAAT batch rembg belum selesai / sebelum fix `is_blocking()` bikin app kelihatan "macet". **Perlu dikonfirmasi ulang oleh user di sesi berikutnya** kalau masih terlihat salah — belum ada bukti bug konkret yang ditemukan ulang.

### 4.4 BELUM diimplementasikan — sisa gap, urutan kepentingan
1. **Cara membedakan "tool call butuh approval sungguhan" vs "tool call rutin"** — gap PALING PENTING sekarang (lihat §4.1b poin 10). Tanpa ini, `is_blocking()` sengaja dikunci `false` selamanya, artinya §2.2's "klik approve/deny lewat mascot" **tidak pernah terpicu dari event asli**, cuma bisa dites manual. Perlu keputusan user: mascot fully-passive selamanya, atau cari sinyal/scoping yang tepat.
2. **Animasi jalan/lari untuk roaming** — roaming state machine sendiri SUDAH jalan (§4.1b poin 9), tapi masih pakai pose Idle berdiri yang "meluncur" karena frame lari (`idle-run.mp4` 87-239, `terbang.mp4` 90-239) belum diproses. Lihat juga poin 8 di bawah.
3. **Drag-to-reposition** — belum ada, dan sekarang harus terintegrasi dengan roaming (drag harus bisa interupsi/pause `RoamCommand`, lihat `roaming.rs`).
4. **Menu lengkap** (riwayat notifikasi, opsi "diemin"/pause permanen, info "AI berbasis Claude") — baru ada install/uninstall-hooks (§4.1 poin 7) + auto-pause-saat-menu-terbuka (§4.1b poin 9). Daftar item lain masih "belum final" per PRD asli.
5. **Mode kontrol manual WASD** (opt-in) — belum ada.
6. **Terminal-focus** (`terminal_focus.rs`, wmctrl-based, dobel-klik sudah wired di frontend) — kodenya ada, **masih belum pernah dites end-to-end**.
7. Suara.
8. **Frame idle-run.mp4 sisanya (87-239, jalan/lari profil) dan terbang.mp4 sisanya (90-239, jalan setelah landing)** — belum diproses, disimpan cuma sebagai referensi frame-range di §5.1 (file mp4 asli tetap ada di `~/Downloads`, tinggal ekstrak ulang kalau perlu untuk animasi roaming/lari beneran — sekarang jadi lebih relevan karena roaming state machine sudah ada, §4.1b poin 9).
9. Packaging (AppImage/deb).
10. **Skema hook Claude Code** — sebagian besar sudah terverifikasi TIDAK SENGAJA lewat live event asli (§4.1 poin 6: `Stop` dan `PreToolUse` dikonfirmasi nama event asli; sesi 3 menambah temuan bahwa `PreToolUse` fire untuk SEMUA tool call, lihat §4.1b poin 10). Tapi belum systematic — `scripts/dev-dump-hook.sh` masih belum pernah dipakai untuk dump semua event secara terstruktur, dan `eventNameToState`/`map_event_to_state` (provisional) belum di-cross-check penuh terhadap semua nama event asli (terutama yang belum pernah muncul: `SessionStart`, `SessionEnd`, `SubagentStart`, `SubagentStop`, `PostToolUse`, `PostToolUseFailure`, `Notification`).
11. Aset di `src/assets/states/` lumayan besar (**77MB total**, 352 file PNG lintas 9 sequence) — belum dioptimasi (kompresi PNG lebih agresif, atau convert ke WebP/sprite-sheet) dan belum dites dampaknya ke waktu-load app pertama kali / ukuran final kalau di-package.

## 5. Aset visual dari user (Gemini-generated video) — status akhir sesi 2

| File | Isi | Status |
|---|---|---|
| `~/Downloads/tux.png` | PNG transparan, referensi bentuk Tux | Tidak dipakai lagi oleh kode (diganti frame-sequence), dibiarkan di `src/assets/tux.png` sbg referensi visual |
| `~/Downloads/tux.mp4` | Video 10s/240 frame @ 24fps, 8 scene (label teks ke-bakar di video: Thinking/Working/Searching/"Finish Working"/Error/Question/Approval/Notification) | **✅ SELESAI diproses** — 7 scene (semua kecuali Notification) sudah di-rembg + crop + jadi sequence di `src/assets/states/{thinking,working,searching,finished,error,question,approval}/f000.png..` |
| `~/Downloads/idle-run.mp4` | Idle berdiri → nengok → lari (profil, ada efek debu kaki), 240 frame @ 24fps | **Porsi Idle (frame 0-86) ✅ selesai** → `src/assets/states/idle/`. Porsi lari (87-239) belum diproses, untuk roaming animation nanti |
| `~/Downloads/terbang.mp4` | Animasi terbang: takeoff (0-29) → glide (30-83) → landing (84-89) → jalan (90-239), 240 frame @ 24fps | **Porsi takeoff+glide+landing (frame 30-89) ✅ selesai** → `src/assets/states/flying/`, dipakai di `playFlyIn()`. Porsi jalan (90-239) belum diproses |

**Boundary scene persis (dari label teks di video, bukan tebakan)**: Thinking 0-27, Working 28-58, Searching 59-86, Finished 87-119, Error 120-147, Question 148-176, Approval 178-205, Notification 209-233 (tidak dipakai, tidak ada state yang map ke situ), fade-out 234-239 (tidak dipakai). Idle (dari idle-run.mp4): standing+nengok 0-86, transisi ke profil 87-99, jalan profil 100-239 (belum diproses). Terbang: siap-siap/sayap terangkat 0-29, launch 30-38, glide 39-83, landing 84-89, jalan depan 90-119, jalan profil 120-239 (belum diproses).

**Tidak ada aset untuk state `RateLimit`** — reuse sequence Idle + tint oranye dari `stateConfig.ts` (`tired: true` → partikel zzz/sweat). Render sudah dicek ulang normal (lihat §4.3).

### 5.1 Pipeline yang dipakai (kalau perlu diulang / diperluas ke frame lain)
```bash
# ekstrak semua frame video ke PNG:
ffmpeg -i <video>.mp4 -start_number 0 "frame_%04d.png"

# rembg — pakai Python session yang di-reuse (BUKAN CLI per-file, terlalu lambat
# krn reload model tiap kali). ~1.4 detik/frame di CPU laptop ini setelah warmup.
# venv persisten (JANGAN taruh di /tmp, akan hilang saat reboot):
#   python3 -m venv ~/.cache/tuxbuddy-rembg-venv
#   ~/.cache/tuxbuddy-rembg-venv/bin/pip install "rembg[cli]" onnxruntime
# script contoh: lihat pola di riwayat sesi 2 (rembg.remove(im, session=new_session('u2net')))
# model: ~/.u2net/u2net.onnx (176MB, permanen, sudah lengkap ter-download)

# auto-crop tiap sequence state ke UNION bounding-box (supaya tidak jitter antar-frame):
# hitung union bbox (alpha channel) di semua frame satu state, crop semua frame ke box yang sama + padding ~10px
```

### 5.2 Background removal — SELESAI (dulu blocker utama sesi 1, sekarang tidak lagi)
Model `u2net.onnx` sempat macet di 89.6% karena network putus-nyambung (sesi 1) — di sesi 2, network sudah normal, resume download `curl -L -C -` langsung selesai penuh (176MB persis). rembg/u2net menghasilkan removal yang bersih untuk semua 9 sequence (idle/thinking/working/searching/finished/error/question/approval/flying) — dicek visual satu-satu, ada sedikit noise/ghosting minor di beberapa frame (misal smudge transparan kecil di pojok approval) tapi tidak mengganggu.

## 6. Keterbatasan yang sudah diketahui

- ~~**Wayland positioning**: `always_on_top`/`set_position` adalah no-op di native Wayland~~ — **SUDAH DIPERBAIKI sesi 2** lewat auto GDK_BACKEND=x11, lihat §4.1 poin 1.
- ~~**Claude tidak bisa screenshot window native**~~ — **KLAIM INI SALAH**, sudah dikoreksi di §4.1 poin 2. `import -window <id>` (ImageMagick) bekerja begitu window jalan lewat XWayland.
- **Terminal-focus**: `wmctrl`/`xdotool` cuma jalan di X11 — sekarang OTOMATIS terpenuhi karena app selalu dipaksa lewat XWayland (GDK_BACKEND=x11), tapi fitur ini sendiri (dobel-klik → fokus terminal) **belum pernah dites end-to-end**, cuma kodenya ada.
- **Skema hook Claude Code**: sebagian besar (`Stop`, `PreToolUse`) sekarang **terverifikasi asli** lewat live event (§4.1 poin 6), tapi belum systematic/lengkap — lihat §4.4 poin 9.
- **`xdotool` synthetic click ke webview kadang tidak konsisten** — percobaan pertama klik-kanan native window sempat tidak terdeteksi (mungkin krn window belum fokus/settle), baru berhasil setelah `wmctrl -a` + delay + retry. Kalau mau otomasi native window lagi, selalu `wmctrl -a <title>` dulu dan kasih jeda sebelum klik.
- **Ukuran aset 77MB** — belum masalah nyata, tapi belum dites dampaknya ke waktu boot/bundle size kalau di-package (AppImage/deb, §4.4 poin 8 masih belum ada).

## 7. Konteks: kenapa proyek ini ada
User tertarik ke project macOS **Coucou** (github.com/Louis-CFM/coucou) — menu-bar companion yang nunjukin karakter animasi reflect state sesi Claude Code, bisa approve/deny permission langsung dari situ. Sudah diverifikasi aman (tidak ada malware, kode MIT tapi aset/karakter/suara berlisensi "all rights reserved" jadi TIDAK boleh dipakai ulang) tapi **tidak bisa jalan di Linux** (native macOS). TuxBuddy adalah reimplementasi konsepnya untuk Linux dengan maskot Tux sendiri, dan sudah berkembang jadi konsep yang lebih ambisius (desktop pet yang roam + fly, bukan sekadar bar).

## 8. Prioritas kerja sesi berikutnya (urutan disarankan, ditulis ulang akhir sesi 3)

1. **Animasi jalan untuk roaming** (§4.4 poin 2/8) — roaming state machine (posisi/gerakan) SUDAH SELESAI sesi 3, tapi masih pakai pose diam yang meluncur karena frame lari belum diproses. Proses `idle-run.mp4` 87-239 dan `terbang.mp4` 90-239 begitu ini jadi prioritas.
2. **Drag-to-reposition** (§4.4 poin 3) — sekarang harus terintegrasi dgn `roaming.rs`'s `RoamCommand` (drag = kirim `Pause`, lepas = `Resume` dari posisi baru — pola yang sama persis dengan yang dipakai utk hooksPanel, lihat `commands::pause_roaming`/`resume_roaming`).
3. **Lengkapi menu** (§4.4 poin 4): riwayat notifikasi, opsi "diemin" PERMANEN (bukan cuma auto-pause saat menu terbuka yang sudah ada), info "AI berbasis Claude".
4. **WASD opt-in** (§4.4 poin 5).
5. Tes `terminal_focus.rs` end-to-end (§4.4 poin 6).
6. **Jalankan `scripts/dev-dump-hook.sh` secara sistematis** (§4.4 poin 10) — pasang di `.claude/settings.json` project SCRATCH (bukan punya user), picu semua event sekali biar `eventNameToState`/`map_event_to_state` bisa dikoreksi penuh.
7. Optimasi ukuran aset (§4.4 poin 11) kalau mulai terasa berat.
8. Suara + packaging (§4.4 poin 7/9) — paling akhir.
9. **(Rendah/opsional)** Scoping approval yang lebih pintar (§4.1b poin 10) — user sudah konfirmasi ini TIDAK urgent selama dia pakai auto-approval/bypass-permissions; cuma relevan digali ulang kalau nanti dia ganti ke mode permission yang lebih ketat.

**Housekeeping sesi berikutnya**:
- Dev server (`npm run tauri dev`) kemungkinan mati sendiri saat reboot user — tinggal `npm run tauri dev` lagi dari awal.
- **PENTING (bug §4.1b poin 10 sempat kelihatan "belum fix" gara-gara ini)**: `target/debug/tuxbuddy-hook` (binary yang beneran dipanggil `~/.claude/settings.json`) itu **binary terpisah**, TIDAK ikut ter-rebuild otomatis oleh `tauri dev`'s file-watcher. Kalau ubah apapun di `crates/protocol` atau `crates/hook-cli`, **selalu** `cargo build --bin tuxbuddy-hook` manual juga sebelum menganggap fix-nya aktif.
- Ubah `src-tauri/tauri.conf.json` (termasuk ukuran window) butuh **restart penuh** `npm run tauri dev` (kill + jalankan ulang) — file-watcher-nya cuma reload untuk perubahan source Rust, dan Vite HMR cuma untuk frontend; config Tauri sendiri tidak ter-hot-reload.
- Project ini **belum git-init**, semua masih working-tree biasa — pertimbangkan `git init` di sesi berikutnya kalau user mau riwayat perubahan mulai terjaga.
