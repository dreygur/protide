//! Base64 encode/decode helpers (no external crate dependency)

/// Simple base64 encoder — returns an `impl Write` that appends base64 chars to `output`.
pub(super) fn base64_encoder(output: &mut Vec<u8>) -> impl std::io::Write + '_ {
    struct Base64Writer<'a>(&'a mut Vec<u8>);
    impl std::io::Write for Base64Writer<'_> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            const CHARS: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            for chunk in buf.chunks(3) {
                let b0 = chunk[0] as usize;
                let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
                let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

                self.0.push(CHARS[b0 >> 2]);
                self.0.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)]);
                if chunk.len() > 1 {
                    self.0.push(CHARS[((b1 & 0x0F) << 2) | (b2 >> 6)]);
                } else {
                    self.0.push(b'=');
                }
                if chunk.len() > 2 {
                    self.0.push(CHARS[b2 & 0x3F]);
                } else {
                    self.0.push(b'=');
                }
            }
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    Base64Writer(output)
}

/// Simple base64 decoder.
pub(super) fn base64_decode(input: &str) -> Option<String> {
    const DECODE: [i8; 128] = {
        let mut t = [-1i8; 128];
        let mut i = 0u8;
        while i < 26 {
            t[(b'A' + i) as usize] = i as i8;
            i += 1;
        }
        i = 0;
        while i < 26 {
            t[(b'a' + i) as usize] = (26 + i) as i8;
            i += 1;
        }
        i = 0;
        while i < 10 {
            t[(b'0' + i) as usize] = (52 + i) as i8;
            i += 1;
        }
        t[b'+' as usize] = 62;
        t[b'/' as usize] = 63;
        t
    };

    let bytes: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'=' && (b as usize) < 128 && DECODE[b as usize] >= 0)
        .collect();

    let mut output = Vec::new();
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let b0 = DECODE[chunk[0] as usize] as u8;
        let b1 = DECODE[chunk[1] as usize] as u8;
        output.push((b0 << 2) | (b1 >> 4));

        if chunk.len() > 2 {
            let b2 = DECODE[chunk[2] as usize] as u8;
            output.push((b1 << 4) | (b2 >> 2));

            if chunk.len() > 3 {
                let b3 = DECODE[chunk[3] as usize] as u8;
                output.push((b2 << 6) | b3);
            }
        }
    }

    String::from_utf8(output).ok()
}
