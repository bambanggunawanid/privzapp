// Indonesian SEO copy: page title, meta description and FAQ for every
// tool. Included by seo.rs.
//
// These are translations of the English entries, not different pages —
// hreflang pairs must say the same thing in both languages. Same length
// rules apply (title 25-65 chars, description 80-165) and a test
// enforces them, because a description that gets truncated in the search
// result is worse than a short one.
//
// Format tokens (PDF, JPG, ZIP, OCR, AES-256) stay untranslated: those
// are exactly the words Indonesian users type into a search box.

/// One tool's localized SEO copy: title, description, FAQ pairs.
pub(crate) type LocalizedSeo = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

/// (slug, title, description, &[(question, answer)])
pub(crate) const SEO_ID: &[LocalizedSeo] = &[
    (
        "edit-pdf",
        "Edit PDF — Tanda Tangan & Gambar Gratis | PrivZapp",
        "Edit PDF gratis di browser: tanda tangan manual, gambar bebas, stempel gambar, putar, nomor halaman, watermark, dan susun ulang halaman. Tanpa unggah.",
        &[
            (
                "Bagaimana cara menandatangani PDF dengan tulisan tangan saya?",
                "Buka PDF di editor, pilih alat pena, lalu gambar tanda tangan Anda langsung di halaman memakai mouse, jari, atau stylus. Klik Terapkan dan unduh PDF yang sudah ditandatangani.",
            ),
            (
                "Bisakah saya menambahkan gambar atau stempel ke PDF?",
                "Bisa — pilih Tambah gambar, pilih fotonya, lalu tarik sebuah kotak di halaman tempat gambar itu diletakkan. Gambar akan ditanam ke dalam PDF saat Anda menerapkannya.",
            ),
            (
                "Apakah menandatangani di sini aman untuk kontrak?",
                "Lebih aman daripada editor berbasis unggahan: dokumen, tanda tangan, dan semua yang Anda gambar tetap di perangkat Anda. Tidak ada salinan di server yang bisa bocor.",
            ),
            (
                "Alat pengeditan apa saja yang tersedia?",
                "Pena dan tanda tangan, kotak teks, stempel gambar, putar halaman, nomor halaman, watermark teks, pangkas margin, susun ulang/hapus/duplikat halaman, tambahkan PDF lain, serta ekspor biasa, terkompres, atau terkunci sandi AES-256.",
            ),
        ],
    ),
    (
        "merge-pdf",
        "Gabung PDF — Satukan File PDF Gratis | PrivZapp",
        "Gabungkan beberapa file PDF menjadi satu dokumen, gratis dan langsung di browser. PDF Anda tidak pernah diunggah — semuanya diproses di perangkat Anda.",
        &[
            (
                "Bagaimana cara menggabungkan beberapa PDF jadi satu?",
                "Pilih atau seret dua PDF atau lebih, susun sesuai urutan yang Anda mau, lalu klik Gabung PDF. Satu file PDF gabungan langsung bisa diunduh.",
            ),
            (
                "Apakah aman menggabungkan PDF rahasia di sini?",
                "Aman. PrivZapp memproses file dengan WebAssembly di dalam browser Anda. PDF tidak pernah meninggalkan perangkat dan tidak ada server yang bisa menyimpannya.",
            ),
            (
                "Apakah ada batas ukuran file atau jumlah halaman?",
                "Tidak ada batas buatan dan tidak ada versi berbayar — menggabungkan PDF gratis selamanya. Satu-satunya batas nyata adalah memori perangkat Anda.",
            ),
        ],
    ),
    (
        "split-pdf",
        "Pisah PDF — Ambil Halaman PDF Gratis | PrivZapp",
        "Pisahkan PDF secara gratis: ambil rentang halaman seperti 1-3,5 atau pecah setiap halaman menjadi file sendiri — tanpa mengunggah ke server mana pun.",
        &[
            (
                "Bagaimana cara mengambil halaman tertentu dari PDF?",
                "Pilih PDF Anda, ketik rentang halaman seperti 1-3,5, lalu klik Pisah PDF. Kosongkan rentangnya untuk menyimpan setiap halaman sebagai PDF terpisah.",
            ),
            (
                "Apakah dokumen saya diunggah saat dipisah?",
                "Tidak. Pemisahan berjalan sepenuhnya di browser Anda lewat WebAssembly; PDF tidak pernah meninggalkan perangkat.",
            ),
            (
                "Apakah memisah PDF gratis?",
                "Ya — semua alat PrivZapp gratis selamanya, didanai donasi, bukan dari data Anda.",
            ),
        ],
    ),
    (
        "rotate-pdf",
        "Putar Halaman PDF Online Gratis | PrivZapp",
        "Putar halaman PDF sebesar 90, 180, atau 270 derajat lalu simpan hasilnya seketika. Gratis, tanpa watermark, dan file tidak pernah meninggalkan perangkat.",
        &[
            (
                "Bagaimana cara memutar PDF lalu menyimpannya?",
                "Pilih PDF, tentukan 90, 180, atau 270 derajat, klik Putar PDF, dan unduh salinan yang sudah diputar. File aslinya tidak diubah.",
            ),
            (
                "Apakah memutar halaman menurunkan kualitas?",
                "Tidak — memutar hanya mengubah penanda orientasi setiap halaman, jadi teks dan gambar tetap setajam sebelumnya.",
            ),
            (
                "Apakah PDF saya diunggah ke server?",
                "Tidak pernah. PrivZapp berjalan sepenuhnya di browser Anda; tidak ada tempat tujuan bagi dokumen Anda.",
            ),
        ],
    ),
    (
        "compress-pdf",
        "Kompres PDF — Perkecil Ukuran File Gratis | PrivZapp",
        "Kompres file PDF menjadi lebih kecil langsung di browser — gratis, tanpa unggah, tanpa watermark. Cocok untuk lampiran email yang dibatasi ukurannya.",
        &[
            (
                "Bagaimana cara kerja kompresi PDF di sini?",
                "PrivZapp memampatkan ulang isi internal PDF Anda dan membuang objek yang tidak terpakai. Jika hasilnya justru lebih besar daripada aslinya, Anda menerima file asli kembali — tidak pernah file yang lebih buruk.",
            ),
            (
                "Apakah kualitas PDF saya menurun?",
                "Tidak. Kompresinya lossless: teks tetap bisa diseleksi dan gambar tidak dikodekan ulang.",
            ),
            (
                "Apakah aman untuk kontrak dan dokumen pribadi?",
                "Aman. File diproses di perangkat Anda dan tidak pernah diunggah, jadi tidak ada orang lain yang bisa melihatnya.",
            ),
        ],
    ),
    (
        "images-to-pdf",
        "JPG ke PDF — Ubah Gambar jadi PDF Gratis | PrivZapp",
        "Ubah JPG, PNG, WebP, dan gambar lain menjadi satu PDF, gratis dan privat. Foto serta hasil pindai menjadi satu PDF tanpa pernah diunggah ke server.",
        &[
            (
                "Bagaimana cara mengubah banyak foto menjadi satu PDF?",
                "Pilih atau seret gambar Anda sesuai urutan yang diinginkan, atur kualitasnya bila perlu, lalu klik Gambar ke PDF. Setiap gambar menjadi satu halaman.",
            ),
            (
                "Format gambar apa saja yang bisa dijadikan PDF?",
                "JPG, PNG, WebP, GIF, BMP, TIFF, dan lainnya — apa pun yang bisa dibaca PrivZapp akan ditanam ke dalam PDF.",
            ),
            (
                "Apakah foto saya diunggah ke suatu tempat?",
                "Tidak. Konversi berlangsung di browser Anda dengan WebAssembly, jadi foto pribadi dan dokumen hasil pindai tetap di perangkat Anda.",
            ),
        ],
    ),
    (
        "watermark-pdf",
        "Watermark PDF — Tambah Teks Gratis | PrivZapp",
        "Bubuhkan watermark teks seperti RAHASIA atau DRAF di seluruh halaman PDF — gratis, seketika, dan sepenuhnya privat karena tidak ada unggahan sama sekali.",
        &[
            (
                "Bagaimana cara menambahkan watermark ke PDF?",
                "Pilih PDF, ketik teks watermark Anda, lalu klik Watermark PDF. Teksnya dicetak menyilang di tengah setiap halaman.",
            ),
            (
                "Bisakah orang lain melihat dokumen yang saya beri watermark?",
                "Tidak — watermark diterapkan di perangkat Anda. File tidak pernah dikirim ke mana pun.",
            ),
            (
                "Apakah alat watermark ini gratis?",
                "Ya, gratis selamanya, tanpa batas masa coba dan tanpa trik menempelkan watermark tambahan milik kami.",
            ),
        ],
    ),
    (
        "reorder-pdf",
        "Susun Ulang Halaman PDF Online Gratis | PrivZapp",
        "Atur ulang urutan halaman PDF, duplikat halaman, atau buang halaman yang tidak diperlukan — gratis dan privat, langsung di dalam browser Anda.",
        &[
            (
                "Bagaimana cara mengubah urutan halaman PDF?",
                "Pilih PDF lalu ketik urutan barunya, misalnya 3,1,2. Ulangi nomor halaman untuk menduplikatnya, atau hilangkan nomor untuk membuang halaman itu.",
            ),
            (
                "Bisakah saya menduplikat sebuah halaman di dalam PDF?",
                "Bisa — tulis nomor halamannya dua kali (misalnya 1,1,2) dan halaman itu akan muncul dua kali di hasilnya.",
            ),
            (
                "Apakah PDF saya disimpan di server setelahnya?",
                "Tidak. Tidak ada server: penyusunan ulang berjalan sepenuhnya di perangkat Anda dan tidak ada yang disimpan.",
            ),
        ],
    ),
    (
        "page-numbers-pdf",
        "Tambah Nomor Halaman PDF Online Gratis | PrivZapp",
        "Tambahkan nomor halaman ke setiap halaman PDF secara gratis — dicetak sebagai halaman / total di bagian bawah, di browser, tanpa unggahan ke mana pun.",
        &[
            (
                "Bagaimana cara menambahkan nomor halaman ke PDF?",
                "Pilih PDF lalu klik Tambah Nomor Halaman. Setiap halaman mendapat label halaman / total di tengah margin bawah.",
            ),
            (
                "Apakah nomornya akan menimpa isi dokumen saya?",
                "Nomor diletakkan di margin bawah dengan ukuran kecil, di bawah area yang biasanya dipakai isi dokumen.",
            ),
            (
                "Apakah dokumen saya diunggah untuk diberi nomor?",
                "Tidak — penomoran berjalan di perangkat Anda lewat WebAssembly; PDF tidak pernah keluar dari browser.",
            ),
        ],
    ),
    (
        "crop-pdf",
        "Pangkas PDF — Potong Margin Halaman Gratis | PrivZapp",
        "Pangkas halaman PDF dengan memotong margin dari sisi mana pun — gratis, seketika, dan privat. Cocok untuk membuang ruang kosong sebelum mencetak.",
        &[
            (
                "Bagaimana cara memangkas margin PDF?",
                "Isi berapa poin yang dipotong dari kiri, atas, kanan, dan bawah (72 poin = 1 inci) lalu klik Pangkas PDF. Semua halaman dipangkas dengan ukuran yang sama.",
            ),
            (
                "Apakah memangkas menghapus isi halaman?",
                "Isi di luar kotak halaman yang baru disembunyikan, bukan dihancurkan — tetapi memangkas ulang dengan nilai nol tidak akan mengembalikannya di alat ini, jadi simpan file asli Anda.",
            ),
            (
                "Apakah PDF diunggah saat dipangkas?",
                "Tidak. Pemangkasan diterapkan di browser Anda dan file tidak pernah meninggalkan perangkat.",
            ),
        ],
    ),
    (
        "extract-text-pdf",
        "PDF ke Teks — Ambil Teks dari PDF Gratis | PrivZapp",
        "Ambil seluruh teks dari sebuah PDF menjadi berkas .txt biasa, gratis dan di dalam browser. Tidak ada unggahan — ideal untuk dokumen rahasia.",
        &[
            (
                "Bagaimana cara mengambil teks dari PDF?",
                "Pilih PDF lalu klik PDF ke Teks. Anda akan mengunduh berkas .txt berisi teks dari setiap halaman.",
            ),
            (
                "Kenapa muncul pesan tidak ada teks yang bisa diambil?",
                "Kemungkinan besar PDF Anda berisi gambar hasil pindai, bukan teks digital. Untuk itu gunakan alat OCR PDF, yang membaca teks langsung dari gambar halaman.",
            ),
            (
                "Apakah ini aman untuk kontrak dan berkas pribadi?",
                "Aman — pengambilan teks berjalan sepenuhnya di perangkat Anda. Tidak ada server yang pernah melihat dokumen itu.",
            ),
        ],
    ),
    (
        "pdf-to-images",
        "PDF ke JPG — Ubah Halaman PDF jadi Gambar | PrivZapp",
        "Ubah halaman PDF menjadi gambar JPG, PNG, atau WebP gratis di browser. Pilih resolusi dan rentang halaman. PDF Anda tidak pernah diunggah ke server.",
        &[
            (
                "Bagaimana cara mengubah PDF menjadi JPG?",
                "Masukkan PDF, pilih JPG sebagai formatnya beserta skala render, lalu jalankan. Setiap halaman kembali sebagai gambar tersendiri — satu halaman diunduh sebagai satu berkas, beberapa halaman datang sebagai .zip.",
            ),
            (
                "Berapa resolusi gambar yang dihasilkan?",
                "Skala 1x memakai resolusi asli PDF yaitu 72 DPI; 2x, 3x, dan 4x mengalikannya (2x = 144 DPI, 4x = 288 DPI). Skala lebih tinggi menghasilkan gambar lebih tajam dan berkas lebih besar — 2x cocok untuk layar, 3x-4x untuk cetak atau OCR.",
            ),
            (
                "Bisakah saya mengubah sebagian halaman saja?",
                "Bisa. Kosongkan kolom halaman untuk mengubah seluruh dokumen, atau ketik rentang seperti 1-3,5 untuk memilih halaman yang Anda inginkan.",
            ),
            (
                "Apakah PDF saya diunggah untuk diubah?",
                "Tidak. Halaman dirender oleh browser Anda sendiri dan gambarnya dikemas di perangkat Anda — berkasnya tidak pernah keluar, dan alat ini tetap berjalan saat offline.",
            ),
        ],
    ),
    (
        "repair-pdf",
        "Perbaiki PDF — Betulkan PDF Rusak Gratis | PrivZapp",
        "Coba perbaiki PDF yang rusak dengan menyusun ulang struktur dan tabel referensi silangnya — gratis, di perangkat Anda, tanpa unggahan apa pun.",
        &[
            (
                "Bagaimana cara kerja perbaikan PDF?",
                "PrivZapp membaca ulang berkas secara longgar, menyelamatkan setiap objek yang masih terbaca, menomori ulang, lalu menulis struktur PDF baru yang bersih.",
            ),
            (
                "Apakah semua PDF rusak bisa diperbaiki?",
                "Tidak — jika data halamannya sendiri sudah hancur, tidak ada alat yang bisa memulihkannya. Kerusakan struktur (offset salah, xref terpotong) biasanya masih bisa.",
            ),
            (
                "Apakah berkas rusak saya diunggah untuk diperbaiki?",
                "Tidak. Perbaikan berlangsung di browser Anda; berkas tidak pernah meninggalkan perangkat.",
            ),
        ],
    ),
    (
        "protect-pdf",
        "Proteksi PDF — Beri Sandi PDF Gratis | PrivZapp",
        "Kunci PDF dengan enkripsi standar AES-256 yang bisa dibuka di pembaca PDF mana pun — gratis, dan kata sandi tidak pernah meninggalkan perangkat Anda.",
        &[
            (
                "Bagaimana cara memberi kata sandi pada PDF?",
                "Pilih PDF, masukkan kata sandi, lalu klik Proteksi PDF. Hasilnya adalah PDF terenkripsi standar yang bisa dibuka pembaca PDF modern mana pun dengan sandi tersebut.",
            ),
            (
                "Seberapa kuat proteksinya?",
                "AES-256 (keamanan standar PDF 2.0). Tidak seperti banyak situs lain, kata sandi dan berkas Anda tidak pernah dikirim — enkripsi terjadi di perangkat Anda.",
            ),
            (
                "Bagaimana jika saya lupa kata sandi PDF-nya?",
                "Tidak ada pintu belakang. Simpan kata sandi Anda baik-baik — tanpa itu berkasnya tetap terkunci.",
            ),
        ],
    ),
    (
        "unlock-pdf",
        "Buka Kunci PDF — Hapus Sandi PDF Gratis | PrivZapp",
        "Hapus kata sandi yang Anda ketahui dari sebuah PDF dan simpan salinan terbuka, gratis di browser — berkas dan sandinya tidak pernah diunggah.",
        &[
            (
                "Bagaimana cara menghapus kata sandi dari PDF?",
                "Pilih PDF yang terkunci, ketik kata sandinya, lalu klik Buka Kunci PDF. Anda mengunduh salinan yang terbuka tanpa perlu sandi.",
            ),
            (
                "Bisakah ini membobol PDF yang sandinya saya lupa?",
                "Tidak — Anda harus tahu sandinya. PrivZapp menghapus proteksi yang memang berhak Anda hapus; alat ini tidak membobol enkripsi.",
            ),
            (
                "Apakah aman memasukkan kata sandi saya di sini?",
                "Aman. Kata sandi hanya dipakai di dalam browser Anda dan tidak pernah dikirim — tidak ada server yang terlibat sama sekali.",
            ),
        ],
    ),
    (
        "convert-img",
        "Konversi Gambar — PNG, JPG, WebP Gratis | PrivZapp",
        "Konversi gambar antara PNG, JPG, WebP, GIF, BMP, TIFF, ICO, dan QOI gratis di browser. Tanpa unggah, tanpa akun, tanpa permainan kualitas.",
        &[
            (
                "Bagaimana cara mengubah PNG ke JPG (atau JPG ke PNG)?",
                "Pilih gambar Anda, tentukan format tujuan dari daftar, atur kualitas untuk format lossy, lalu klik Konversi Gambar. Setiap berkas diunduh dalam format barunya.",
            ),
            (
                "Format apa saja yang didukung?",
                "PNG, JPG, WebP, GIF, BMP, TIFF, ICO, dan QOI — bebas dari mana pun ke mana pun, termasuk konversi banyak berkas sekaligus.",
            ),
            (
                "Apakah gambar saya diunggah saat dikonversi?",
                "Tidak pernah. Pengonversinya adalah WebAssembly yang berjalan di browser Anda, jadi foto tetap di perangkat.",
            ),
        ],
    ),
    (
        "resize-img",
        "Ubah Ukuran Gambar Gratis — Piksel Persis | PrivZapp",
        "Ubah ukuran gambar ke dimensi piksel yang persis atau skalakan dengan rasio tetap terjaga — gratis, privat, di browser Anda, tanpa unggahan.",
        &[
            (
                "Bagaimana cara mengubah ukuran gambar ke dimensi tertentu?",
                "Masukkan lebar dan tinggi dalam piksel lalu klik Ubah Ukuran Gambar. Kosongkan salah satu kolom agar rasio aspeknya terjaga otomatis.",
            ),
            (
                "Apakah mengubah ukuran menurunkan kualitas?",
                "PrivZapp memakai resampling Lanczos berkualitas tinggi, filter yang sama dipakai perkakas profesional, sehingga gambar yang diperkecil tetap tajam.",
            ),
            (
                "Bisakah saya mengubah ukuran banyak gambar sekaligus?",
                "Bisa — pilih beberapa berkas sekaligus dan semuanya diubah dengan pengaturan yang sama dalam sekali klik.",
            ),
        ],
    ),
    (
        "compress-img",
        "Kompres Gambar — Perkecil JPG & PNG Gratis | PrivZapp",
        "Kompres gambar JPG dan PNG menjadi berkas lebih kecil secara gratis, lengkap dengan penggeser kualitas — diproses di browser, tidak pernah diunggah.",
        &[
            (
                "Bagaimana cara memperkecil ukuran berkas gambar?",
                "Pilih gambar Anda, atur penggeser kualitas, lalu klik Kompres Gambar. Jika kompresi tidak bisa mengalahkan ukuran asli, Anda menerima berkas asli kembali — tidak pernah berkas yang lebih besar.",
            ),
            (
                "Berapa nilai kualitas yang paling pas?",
                "Nilai 80 adalah pilihan awal yang bagus untuk foto: berkas jauh lebih kecil dengan perbedaan yang hampir tidak terlihat. Turunkan untuk thumbnail, naikkan untuk cetak.",
            ),
            (
                "Apakah pengompres gambar ini benar-benar privat?",
                "Ya. Kompresi berjalan di perangkat Anda lewat WebAssembly. Foto Anda tidak pernah dikirim ke server.",
            ),
        ],
    ),
    (
        "rotate-img",
        "Putar Gambar Online Gratis — JPG, PNG, dll | PrivZapp",
        "Putar foto sebesar 90, 180, atau 270 derajat, gratis dan di dalam browser. Putar banyak gambar sekaligus; tidak ada yang pernah diunggah.",
        &[
            (
                "Bagaimana cara memutar foto lalu menyimpannya?",
                "Pilih gambar Anda, tentukan sudutnya, lalu klik Putar Gambar. Setiap salinan yang sudah diputar langsung diunduh dalam format aslinya.",
            ),
            (
                "Apakah memutar gambar menurunkan kualitas?",
                "Rotasi 90/180/270 derajat memetakan ulang piksel tanpa resampling; hanya format lossy seperti JPG yang dikodekan ulang, pada kualitas yang Anda tentukan.",
            ),
            (
                "Bisakah saya memutar banyak gambar sekaligus?",
                "Bisa — pilih berapa pun jumlah gambar dan semuanya diputar dengan sudut yang sama dalam sekali klik.",
            ),
        ],
    ),
    (
        "flip-img",
        "Balik Gambar — Cerminkan Foto Online Gratis | PrivZapp",
        "Cerminkan gambar secara mendatar atau tegak, gratis dan privat. Perbaiki swafoto dan hasil pindai di browser — foto tidak pernah keluar dari perangkat.",
        &[
            (
                "Bagaimana cara mencerminkan sebuah foto?",
                "Pilih gambarnya, tentukan mendatar (kiri ke kanan) atau tegak (atas ke bawah), lalu klik Balik Gambar.",
            ),
            (
                "Kenapa swafoto perlu dibalik?",
                "Kamera depan biasanya menyimpan pratinjau yang tercermin; membalik secara mendatar mengembalikan wajah Anda seperti yang dilihat orang lain.",
            ),
            (
                "Apakah foto saya diunggah untuk dibalik?",
                "Tidak — pembalikan terjadi di perangkat Anda lewat WebAssembly dan berkasnya tidak pernah meninggalkan browser.",
            ),
        ],
    ),
    (
        "upscale-img",
        "Perbesar Gambar — Perbesar 2x atau 4x Gratis | PrivZapp",
        "Perbesar gambar 2x atau 4x dengan resampling Lanczos yang tajam, gratis di browser. Tanpa akun, tanpa unggah, tanpa watermark pada hasilnya.",
        &[
            (
                "Bagaimana cara memperbesar gambar tanpa terlalu buram?",
                "Pilih gambarnya, tentukan 2x atau 4x, lalu klik Perbesar Gambar. PrivZapp memakai resampling Lanczos — filter pembesaran klasik yang paling tajam.",
            ),
            (
                "Apakah ini pembesaran berbasis AI?",
                "Bukan — ini resampling klasik berkualitas tinggi yang berjalan seketika di perangkat Anda. Pembesar AI mengarang detail baru; alat ini memperbesar dengan setia apa yang memang ada.",
            ),
            (
                "Apakah gambar saya diunggah untuk diperbesar?",
                "Tidak pernah. Pembesaran berjalan di browser Anda, jadi foto pribadi tetap pribadi.",
            ),
        ],
    ),
    (
        "grayscale-img",
        "Gambar Hitam Putih — Pengubah Gratis | PrivZapp",
        "Ubah foto menjadi hitam putih gratis di browser. Grayscale berbasis luminansi sungguhan, mendukung banyak berkas, dan tidak ada yang pernah diunggah.",
        &[
            (
                "Bagaimana cara membuat foto jadi hitam putih?",
                "Pilih gambar Anda lalu klik Gambar Hitam Putih. Setiap gambar diubah memakai pembobotan luminansi yang tepat dan diunduh dalam format aslinya.",
            ),
            (
                "Apakah transparansi tetap terjaga setelah diubah?",
                "Ya — kanal alfa dipertahankan untuk format yang mendukungnya, seperti PNG dan WebP.",
            ),
            (
                "Apakah pengubah ini privat?",
                "Sepenuhnya: pengubahan berjalan di perangkat Anda dan foto tidak pernah sampai ke server.",
            ),
        ],
    ),
    (
        "blur-img",
        "Buramkan Gambar Online Gratis — Blur Gaussian | PrivZapp",
        "Buramkan gambar dengan kekuatan gaussian yang bisa diatur, gratis di browser. Haluskan tangkapan layar atau latar tanpa mengunggah apa pun.",
        &[
            (
                "Bagaimana cara membuat foto menjadi buram?",
                "Pilih gambarnya, atur penggeser kekuatan, lalu klik Buramkan Gambar. Kekuatan lebih tinggi berarti hasil yang lebih lembut dan menyebar.",
            ),
            (
                "Apakah blur bisa diandalkan untuk menyembunyikan teks sensitif?",
                "Blur yang kuat membuat teks tidak terbaca, tetapi untuk penyensoran sungguhan sebaiknya potong saja bagian itu — blur kadang bisa dipulihkan sebagian.",
            ),
            (
                "Apakah gambar saya diunggah untuk diburamkan?",
                "Tidak — blur dihitung di perangkat Anda; gambarnya tidak pernah meninggalkan browser.",
            ),
        ],
    ),
    (
        "watermark-img",
        "Watermark Gambar — Tambah Teks ke Foto Gratis | PrivZapp",
        "Bubuhkan teks semi transparan di seluruh foto Anda untuk melindunginya, gratis di browser — gambar tidak pernah diunggah ke mana pun.",
        &[
            (
                "Bagaimana cara memberi watermark pada gambar?",
                "Pilih gambar Anda, ketik teks watermark, lalu klik Watermark Gambar. Teksnya dicetak semi transparan melintang di bagian tengah.",
            ),
            (
                "Bisakah saya memberi watermark ke banyak foto sekaligus?",
                "Bisa — pilih sekumpulan berkas dan setiap gambar mendapat stempel yang sama dalam sekali klik.",
            ),
            (
                "Apakah foto saya diunggah untuk diberi watermark?",
                "Tidak. Pemberian watermark berjalan di perangkat Anda, jadi karya yang belum dipublikasikan tetap dalam kendali Anda.",
            ),
        ],
    ),
    (
        "strip-exif",
        "Hapus Data EXIF — Bersihkan Metadata Foto | PrivZapp",
        "Hapus metadata EXIF — lokasi GPS, model kamera, waktu pengambilan — dari foto sebelum dibagikan. Gratis dan privat: foto tidak keluar dari perangkat.",
        &[
            (
                "Kenapa saya perlu menghapus data EXIF sebelum membagikan foto?",
                "Foto sering menyimpan koordinat GPS rumah Anda, nomor seri kamera, dan waktu pengambilan yang persis. Menghapus metadata membuang informasi tersembunyi itu.",
            ),
            (
                "Apakah menghapus metadata mengubah tampilan foto?",
                "Pikselnya tetap dipertahankan; gambar dikodekan ulang tanpa blok metadata apa pun. Untuk JPG, Anda mengatur kualitas pengodean ulang lewat penggeser.",
            ),
            (
                "Apakah penghapus EXIF ini sendiri privat?",
                "Sepenuhnya — dan itulah intinya. Foto dibersihkan di perangkat Anda dan tidak pernah diunggah, tidak seperti layanan web yang harus melihat foto Anda untuk membersihkannya.",
            ),
        ],
    ),
    (
        "crop-img",
        "Potong Gambar Online Gratis — Tanpa Unggah | PrivZapp",
        "Potong gambar ke kotak piksel yang persis, gratis di browser. Potong banyak gambar dengan bingkai yang sama sekaligus, tanpa unggahan apa pun.",
        &[
            (
                "Bagaimana cara memotong gambar dengan ukuran piksel persis?",
                "Masukkan posisi X/Y sudut kiri atas serta lebar dan tinggi yang ingin dipertahankan, lalu klik Potong Gambar.",
            ),
            (
                "Bisakah saya memotong beberapa gambar dengan cara yang sama?",
                "Bisa — pilih beberapa gambar dan kotak yang sama diterapkan ke masing-masing, cocok untuk tangkapan layar atau hasil pindai dengan tata letak identik.",
            ),
            (
                "Apakah alat potong ini gratis dan privat?",
                "Keduanya: gratis selamanya, dan gambar Anda hanya diproses di perangkat Anda sendiri.",
            ),
        ],
    ),
    (
        "favicon-pack",
        "Pembuat Favicon — PNG/JPG ke Paket ICO | PrivZapp",
        "Ubah PNG atau JPG apa pun menjadi paket favicon lengkap: favicon.ico, beragam ukuran PNG, apple-touch-icon, dan webmanifest dalam satu ZIP — tanpa unggah.",
        &[
            (
                "Apa saja isi paket favicon ini?",
                "favicon.ico multi ukuran (16/32/48), favicon-16x16.png, favicon-32x32.png, apple-touch-icon.png (180), android-chrome 192 dan 512, sebuah site.webmanifest, serta README berisi potongan HTML yang tinggal ditempel.",
            ),
            (
                "Bagaimana cara memasang favicon ke situs saya?",
                "Ekstrak semuanya ke folder root situs Anda lalu tempel potongan empat baris dari README.txt ke bagian head halaman. Browser akan mengambil favicon.ico secara otomatis.",
            ),
            (
                "Apakah logo saya diunggah untuk membuat ikonnya?",
                "Tidak — setiap ukuran dibuat di browser Anda dengan WebAssembly. Logo yang belum dirilis tetap di perangkat Anda.",
            ),
        ],
    ),
    (
        "rename-batch",
        "Ganti Nama Berkas Massal Online Gratis | PrivZapp",
        "Ganti nama banyak berkas sekaligus dengan pola seperti liburan-{n} — penomoran otomatis dan ekstensi tetap terjaga. Gratis, privat, tanpa unggah.",
        &[
            (
                "Bagaimana cara kerja penggantian nama berpola?",
                "Ketik pola seperti liburan-{n}; bagian {n} menjadi 1, 2, 3, dan seterusnya sesuai urutan. Tanpa {n}, nomor tetap ditambahkan otomatis agar nama tidak bentrok.",
            ),
            (
                "Apakah ekstensi berkas ikut berubah?",
                "Tidak — setiap berkas mempertahankan ekstensi aslinya, hanya bagian namanya yang berubah.",
            ),
            (
                "Apakah berkas saya diunggah untuk diganti namanya?",
                "Tidak. Penggantian nama terjadi seketika di browser Anda dan salinannya langsung diunduh kembali kepada Anda.",
            ),
        ],
    ),
    (
        "zip-files",
        "Kompres Berkas ke ZIP Online Gratis | PrivZapp",
        "Kompres berkas menjadi arsip ZIP gratis di browser. Bungkus dokumen, foto, atau apa pun — tidak ada yang pernah diunggah ke server mana pun.",
        &[
            (
                "Bagaimana cara mengompres berkas menjadi ZIP?",
                "Pilih atau seret berkas apa pun lalu klik Buat ZIP. Anda mengunduh satu archive.zip berisi semuanya, terkompresi dengan metode deflate.",
            ),
            (
                "Apakah ada batas jumlah berkas atau ukuran total?",
                "Tidak ada batas buatan — alat ini gratis selamanya. Memori perangkat Anda adalah satu-satunya batas nyata.",
            ),
            (
                "Apakah membuat ZIP di sini lebih aman daripada situs ZIP lain?",
                "Ya: situs ZIP pada umumnya mengunggah berkas Anda untuk mengompresnya. PrivZapp mengompres di perangkat Anda, jadi isinya tetap milik Anda sendiri.",
            ),
        ],
    ),
    (
        "unzip",
        "Ekstrak ZIP Online Gratis — Tanpa Unggah | PrivZapp",
        "Buka dan ekstrak arsip ZIP gratis di browser, lengkap dengan perlindungan zip bomb dan path traversal. Arsipnya tidak pernah keluar dari perangkat Anda.",
        &[
            (
                "Bagaimana cara membuka berkas ZIP tanpa memasang aplikasi?",
                "Letakkan berkas .zip di sini lalu klik Ekstrak ZIP — setiap berkas di dalamnya muncul sebagai unduhan, langsung dari browser Anda.",
            ),
            (
                "Apakah aman mengekstrak ZIP dari sumber yang tidak dikenal?",
                "PrivZapp melindungi dari zip bomb dan nama berkas yang menjebol folder, dan karena tidak ada yang dijalankan atau diunggah, proses ekstraksi tetap dalam kendali Anda.",
            ),
            (
                "Apakah berkas hasil ekstraksi dikirim ke suatu tempat?",
                "Tidak — arsip dibaca di perangkat Anda dan isinya langsung kembali kepada Anda.",
            ),
        ],
    ),
    (
        "encrypt-file",
        "Enkripsi Berkas dengan Sandi — AES-256 | PrivZapp",
        "Kunci berkas apa pun dengan enkripsi AES-256-GCM, gratis dan bisa dipakai offline. Kunci diturunkan di perangkat Anda; tidak ada yang diunggah.",
        &[
            (
                "Seberapa kuat enkripsinya?",
                "AES-256-GCM dengan kunci yang diturunkan dari kata sandi Anda lewat PBKDF2-HMAC-SHA256 sebanyak 600.000 putaran — kelas kriptografi yang sama dengan yang dipakai perbankan.",
            ),
            (
                "Apa itu berkas .pzv?",
                "Itu brankas PrivZapp: berkas Anda dalam keadaan terenkripsi. Buka kapan saja dengan alat Dekripsi Berkas memakai kata sandi yang sama, di perangkat mana pun yang menjalankan PrivZapp.",
            ),
            (
                "Bagaimana jika saya lupa kata sandinya?",
                "Berkasnya tidak bisa dipulihkan — memang begitu rancangannya. Tidak ada siapa pun, termasuk kami, yang bisa membukanya tanpa kata sandi, karena kami tidak pernah melihat berkas maupun sandinya.",
            ),
        ],
    ),
    (
        "decrypt-file",
        "Dekripsi Berkas — Buka Brankas .pzv | PrivZapp",
        "Dekripsi brankas .pzv PrivZapp dengan kata sandi Anda dan dapatkan berkas aslinya kembali — secara lokal di browser, tanpa unggahan apa pun.",
        &[
            (
                "Bagaimana cara membuka berkas .pzv?",
                "Pilih brankas .pzv, masukkan kata sandi yang dipakai saat mengenkripsi, lalu klik Dekripsi Berkas. Berkas aslinya langsung terunduh.",
            ),
            (
                "Muncul pesan sandi salah atau berkas rusak — lalu bagaimana?",
                "AES-GCM memeriksa keutuhan berkas, jadi kemungkinan kata sandinya berbeda dari yang dipakai saat mengenkripsi, atau berkasnya sudah berubah. Periksa kata sandinya lebih dulu.",
            ),
            (
                "Apakah dekripsi terjadi di server?",
                "Tidak — brankas dan kata sandi tidak pernah meninggalkan perangkat Anda. Justru itulah yang membuat enkripsi PrivZapp aman dipakai.",
            ),
        ],
    ),
    (
        "video-to-gif",
        "Video ke GIF — Ubah MP4/WebM jadi GIF | PrivZapp",
        "Ubah klip video menjadi GIF berulang gratis di browser. Pilih laju bingkai, lebar, dan rentang waktu. Video tidak pernah diunggah ke server mana pun.",
        &[
            (
                "Bagaimana cara mengubah video menjadi GIF?",
                "Masukkan klipnya, pilih laju bingkai dan lebar, atur waktu mulai dan selesai bila perlu, lalu jalankan. GIF dibuat di perangkat Anda dengan palet dua tahap sehingga warnanya tetap bersih.",
            ),
            (
                "Kenapa proses pertama lebih lambat daripada berikutnya?",
                "Pengonversinya adalah FFmpeg utuh yang dikompilasi ke WebAssembly (sekitar 10 MB, diunduh sekali lalu disimpan di cache). Setelah pemuatan pertama itu, semuanya berjalan lokal secepat aplikasi biasa — bahkan saat offline.",
            ),
            (
                "Bagaimana caranya agar ukuran GIF tetap kecil?",
                "Turunkan laju bingkai (10-12 fps sudah nyaman dilihat), kecilkan lebarnya, dan persingkat klip lewat rentang waktu — GIF adalah format lama yang ukurannya cepat membengkak.",
            ),
            (
                "Apakah video saya diunggah untuk dikonversi?",
                "Tidak. FFmpeg berjalan di dalam tab browser Anda; berkasnya tidak pernah meninggalkan perangkat dan tidak ada server yang bisa menyimpan salinannya.",
            ),
        ],
    ),
    (
        "trim-video",
        "Potong Video Online Gratis — MP4/WebM | PrivZapp",
        "Potong sebagian video gratis di browser tanpa mengodekan ulang — kualitas asli, cepat. Tidak ada unggahan; berkas tetap di perangkat Anda.",
        &[
            (
                "Bagaimana cara memotong video tanpa kehilangan kualitas?",
                "Masukkan waktu mulai dan selesai lalu jalankan. Alat ini menyalin aliran video dan audio asli ke berkas baru alih-alih mengodekan ulang, jadi kualitasnya persis sama seperti aslinya.",
            ),
            (
                "Kenapa potongannya tidak pas di waktu mulai yang saya isi?",
                "Pemotongan tanpa kehilangan kualitas hanya bisa dimulai pada keyframe, jadi klip menempel ke keyframe terdekat sebelum waktu mulai Anda — biasanya meleset sepersekian detik. Kodekan ulang dengan Konversi Video bila Anda butuh potongan yang persis.",
            ),
            (
                "Apakah video saya diunggah untuk dipotong?",
                "Tidak. Pemotongan terjadi di browser Anda lewat FFmpeg yang dikompilasi ke WebAssembly (diunduh sekali, sekitar 10 MB, lalu disimpan di cache). Video tidak pernah meninggalkan perangkat.",
            ),
        ],
    ),
    (
        "convert-video",
        "Konversi Video — MP4, WebM, MKV, MOV, AVI | PrivZapp",
        "Konversi video antara MP4, WebM, MKV, MOV, dan AVI gratis di browser, lengkap dengan penggeser kualitas. GIF juga bisa jadi video. Tanpa unggahan.",
        &[
            (
                "Format apa saja yang bisa saling dikonversi?",
                "Video apa pun yang bisa dibaca browser Anda — MP4, WebM, MKV, MOV, AVI, dan lainnya — menjadi MP4 (H.264 + AAC), WebM (VP8 + Opus), MKV, MOV, atau AVI. GIF beranimasi juga bisa jadi masukan, sehingga GIF berubah menjadi video sungguhan yang jauh lebih kecil.",
            ),
            (
                "Kenapa proses konversi memakan waktu?",
                "Mengodekan ulang video adalah pekerjaan berat, dan di sini semuanya berjalan di perangkat Anda — FFmpeg utuh yang dikompilasi ke WebAssembly (unduhan sekali sekitar 10 MB), bukan kumpulan server. Video panjang butuh beberapa menit; tidak ada yang pernah diunggah.",
            ),
            (
                "Format mana yang sebaiknya saya pilih?",
                "MP4 (H.264) bisa diputar di mana saja, termasuk perangkat lama. WebM bebas royalti dan sering lebih kecil pada kualitas yang sama, dan semua browser modern bisa memutarnya.",
            ),
        ],
    ),
    (
        "extract-audio",
        "Ambil Audio dari Video — MP3, WAV | PrivZapp",
        "Ambil jalur audio dari video apa pun gratis di browser, sebagai MP3, WAV, OGG, atau M4A. Video tidak pernah diunggah — FFmpeg berjalan di perangkat Anda.",
        &[
            (
                "Bagaimana cara mengambil audio dari sebuah video?",
                "Masukkan videonya, pilih MP3, WAV, OGG, atau M4A, lalu jalankan. Anda bisa mengatur waktu mulai dan selesai untuk mengambil satu bagian saja, dan penggeser kualitas mengatur bitrate untuk format lossy.",
            ),
            (
                "Format mana yang sebaiknya dipilih?",
                "MP3 bisa diputar di mana saja dan merupakan pilihan aman. WAV tidak terkompresi (paling besar, lossless), OGG adalah pilihan perangkat lunak bebas, dan M4A (AAC) lebih disukai perangkat Apple.",
            ),
            (
                "Apakah video saya diunggah untuk diambil audionya?",
                "Tidak. FFmpeg berjalan di dalam tab browser Anda (unduhan sekali sekitar 10 MB, disimpan untuk dipakai offline) — video dan suaranya tidak pernah meninggalkan perangkat.",
            ),
        ],
    ),
    (
        "ocr-pdf",
        "OCR PDF Gratis — PDF Pindaian jadi Teks | PrivZapp",
        "Baca teks dari PDF hasil pindai gratis di browser dengan OCR di perangkat. Pilih bahasa dan rentang halaman — dokumen tidak pernah diunggah.",
        &[
            (
                "Bagaimana cara mengambil teks dari PDF hasil pindai?",
                "Masukkan PDF-nya, pilih bahasanya, lalu jalankan. Setiap halaman dirender dan dikenali di perangkat Anda, dan hasilnya diunduh sebagai berkas .txt biasa lengkap dengan penanda halaman.",
            ),
            (
                "Apa bedanya dengan alat PDF ke Teks?",
                "PDF ke Teks mengambil teks yang memang sudah tersimpan digital di dalam PDF — cepat dan persis, tetapi tidak berguna untuk hasil pindai. OCR PDF benar-benar membaca gambar halamannya, jadi dokumen hasil foto dan pindai pun bisa.",
            ),
            (
                "Seberapa akurat, dan bagaimana meningkatkannya?",
                "Hasil pindai yang bersih terbaca sangat baik. Untuk cetakan samar atau kecil, naikkan pilihan resolusinya — pengenalan dilakukan pada ukuran hasil render, jadi 3x-4x memberi mesin OCR lebih banyak piksel.",
            ),
            (
                "Apakah dokumen saya diunggah untuk di-OCR?",
                "Tidak. Mesin pengenalannya (beberapa MB, diunduh sekali lalu disimpan) berjalan di dalam tab browser Anda. Halaman, piksel, dan teksnya tidak pernah meninggalkan perangkat.",
            ),
        ],
    ),
    (
        "image-to-text",
        "Gambar ke Teks — OCR Gratis di Browser | PrivZapp",
        "Salin teks dari gambar apa pun — foto, tangkapan layar, hasil pindai — dengan OCR gratis di perangkat. Tanpa unggahan; pengenalan berjalan di browser.",
        &[
            (
                "Bagaimana cara mengambil teks dari sebuah gambar?",
                "Masukkan satu atau beberapa gambar, pilih bahasanya, lalu jalankan. Setiap gambar kembali sebagai berkas .txt tersendiri berisi teks yang dikenali sesuai urutan bacanya.",
            ),
            (
                "Gambar seperti apa yang hasilnya paling bagus?",
                "Teks yang tajam dan terang di atas latar polos. Tangkapan layar terbaca nyaris sempurna; untuk foto, potong mendekati teksnya (alat Potong Gambar bisa membantu) dan hindari sudut yang miring.",
            ),
            (
                "Apakah gambar saya diunggah untuk dibaca?",
                "Tidak. Mesin OCR-nya adalah WebAssembly yang berjalan di browser Anda sendiri — unduhan sekali beberapa MB, disimpan untuk dipakai offline. Gambar Anda tidak pernah meninggalkan perangkat.",
            ),
        ],
    ),
];
