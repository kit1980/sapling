use libc::c_void;
use std::cmp;
use std::ffi::CStr;
use std::io;

use zstd_sys::{ZSTD_DCtx_setMaxWindowSize, ZSTD_compressBound, ZSTD_compress_advanced,
               ZSTD_compressionParameters, ZSTD_createCCtx, ZSTD_createDCtx,
               ZSTD_decompress_usingDict, ZSTD_findDecompressedSize, ZSTD_frameParameters,
               ZSTD_freeCCtx, ZSTD_freeDCtx, ZSTD_getErrorName, ZSTD_isError, ZSTD_parameters,
               ZSTD_strategy_ZSTD_fast, ZSTD_CHAINLOG_MIN, ZSTD_CONTENTSIZE_ERROR,
               ZSTD_CONTENTSIZE_UNKNOWN, ZSTD_HASHLOG_MIN, ZSTD_SEARCHLOG_MIN,
               ZSTD_TARGETLENGTH_MIN, ZSTD_WINDOWLOG_MIN};

// They are complex "#define"s that are not exposed by bindgen automatically
const ZSTD_WINDOWLOG_MAX: u32 = 30;
const ZSTD_HASHLOG_MAX: u32 = 30;

/// Return `y` so `1 << y` is greater than `x`.
/// Note: `1 << y` might be greater than `u64::MAX`.
fn log_base2(x: u64) -> u32 {
    64 - x.leading_zeros()
}

/// Adjust a value so it is in the given range.
fn clamp(v: u32, min: u32, max: u32) -> u32 {
    cmp::max(min, cmp::min(v, max))
}

/// Convert zstd error code to a static string.
fn explain_error(code: usize) -> &'static str {
    unsafe {
        // ZSTD_getErrorName returns a static string.
        let name = ZSTD_getErrorName(code);
        let cstr = CStr::from_ptr(name);
        cstr.to_str().expect("zstd error is utf-8")
    }
}

/// Create a "zstd delta". Compress `data` using dictionary `base`.
pub fn diff(base: &[u8], data: &[u8]) -> io::Result<Vec<u8>> {
    // Customized wlog, hlog to let zstd do better at delta-ing. Use "fast" strategy, which is
    // good enough assuming the primary space saving is caused by "delta-ing".
    let log = log_base2((data.len() + base.len() + 1) as u64);
    let wlog = clamp(log, ZSTD_WINDOWLOG_MIN, ZSTD_WINDOWLOG_MAX);
    let hlog = clamp(log, ZSTD_HASHLOG_MIN, ZSTD_HASHLOG_MAX);
    let cparams = ZSTD_compressionParameters {
        windowLog: wlog,
        chainLog: ZSTD_CHAINLOG_MIN, // useless using "fast" strategy
        hashLog: hlog,
        searchLog: ZSTD_SEARCHLOG_MIN, // useless using "fast" strategy
        searchLength: 7,               // level 1 default (see ZSTD_defaultCParameters)
        targetLength: ZSTD_TARGETLENGTH_MIN, // useless for "fast" strategy
        strategy: ZSTD_strategy_ZSTD_fast,
    };
    let fparams = ZSTD_frameParameters {
        contentSizeFlag: 1, // needed by `apply`
        checksumFlag: 0,    // checksum is done at another level
        noDictIDFlag: 1,    // dictionary is fixed, not reused
    };
    let params = ZSTD_parameters {
        cParams: cparams,
        fParams: fparams,
    };

    unsafe {
        let cctx = ZSTD_createCCtx();
        if cctx.is_null() {
            return Err(io::Error::new(io::ErrorKind::Other, "cannot create CCtx"));
        }

        let max_outsize = ZSTD_compressBound(data.len());
        let mut buf: Vec<u8> = Vec::with_capacity(max_outsize);

        buf.set_len(max_outsize);
        let outsize = ZSTD_compress_advanced(
            cctx,
            buf.as_mut_ptr() as *mut c_void,
            buf.len(),
            data.as_ptr() as *const c_void,
            data.len(),
            base.as_ptr() as *const c_void,
            base.len(),
            params,
        );

        ZSTD_freeCCtx(cctx);

        if ZSTD_isError(outsize) != 0 {
            let msg = format!("cannot compress ({})", explain_error(outsize));
            Err(io::Error::new(io::ErrorKind::Other, msg))
        } else {
            buf.set_len(outsize);
            Ok(buf)
        }
    }
}

/// Apply a zstd `delta` generated by `diff` to `base`. Return reconstructed `data`.
pub fn apply(base: &[u8], delta: &[u8]) -> io::Result<Vec<u8>> {
    unsafe {
        let dctx = ZSTD_createDCtx();
        if dctx.is_null() {
            return Err(io::Error::new(io::ErrorKind::Other, "cannot create DCtx"));
        }
        ZSTD_DCtx_setMaxWindowSize(dctx, 1 << ZSTD_WINDOWLOG_MAX);

        let size = ZSTD_findDecompressedSize(delta.as_ptr() as *const c_void, delta.len()) as usize;
        if size == ZSTD_CONTENTSIZE_ERROR as usize || size == ZSTD_CONTENTSIZE_UNKNOWN as usize {
            ZSTD_freeDCtx(dctx);
            let msg = "cannot get decompress size";
            return Err(io::Error::new(io::ErrorKind::Other, msg));
        }

        let mut buf: Vec<u8> = Vec::with_capacity(size);
        buf.set_len(size);

        let outsize = ZSTD_decompress_usingDict(
            dctx,
            buf.as_mut_ptr() as *mut c_void,
            size,
            delta.as_ptr() as *const c_void,
            delta.len(),
            base.as_ptr() as *const c_void,
            base.len(),
        );
        ZSTD_freeDCtx(dctx);

        if ZSTD_isError(outsize) != 0 {
            let msg = format!("cannot decompress ({})", explain_error(outsize));
            Err(io::Error::new(io::ErrorKind::Other, msg))
        } else if outsize != size {
            let msg = format!(
                "decompress size mismatch (expected {}, got {})",
                size, outsize
            );
            Err(io::Error::new(io::ErrorKind::Other, msg))
        } else {
            Ok(buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{ChaChaRng, RngCore, SeedableRng};

    fn check_round_trip(base: &[u8], data: &[u8]) -> bool {
        let delta = diff(base, data).expect("delta");
        let reconstructed = apply(base, &delta).expect("apply");
        reconstructed[..] == data[..]
    }

    #[test]
    fn test_round_trip_manual() {
        assert!(check_round_trip(b"", b""));
        assert!(check_round_trip(b"123", b""));
        assert!(check_round_trip(b"", b"123"));
        assert!(check_round_trip(b"1234567890", b"3"));
        assert!(check_round_trip(b"3", b"1234567890"));
    }

    #[test]
    fn test_delta_efficiency() {
        // 1 MB incompressible random data
        let mut base = vec![0u8; 1000000];
        ChaChaRng::from_seed([0; 32]).fill_bytes(base.as_mut());
        // Change a few bytes
        let mut data = base.clone();
        data[0] ^= 1;
        data[10000] ^= 3;
        data[900000] ^= 7;
        let delta = diff(&base, &data).expect("diff");
        // Should generate a small delta.
        // Note: this will fail if wlog/hlog are not tweaked.
        assert!(delta.len() < 200);
    }

    quickcheck! {
        fn test_round_trip_quickcheck(a: Vec<u8>, b: Vec<u8>) -> bool {
            check_round_trip(&a, &b)
        }
    }
}
