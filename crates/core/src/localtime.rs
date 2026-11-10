//! Local wall-clock time of unix seconds: the one helper behind every HH:MM:SS stamp and record file name.
//! Unix asks libc (localtime_r); Windows asks the OS (SystemTimeToTzSpecificLocalTime, which applies that
//! date's DST rule). Both reduce to a UTC offset, so no time-zone crate is needed.

/// Local [year, month, day, hour, minute, second] of unix seconds `secs`.
pub fn local_civil(secs: i64) -> [i64; 6] {
    offset_civil(secs, local_offset_secs(secs))
}

/// [year, month, day, hour, minute, second] of `secs` shifted by `offset` seconds east of UTC.
pub fn offset_civil(secs: i64, offset: i64) -> [i64; 6] {
    utc_civil(secs + offset)
}

/// Seconds east of UTC in force at unix seconds `secs`. 0 when the OS cannot say.
pub fn local_offset_secs(secs: i64) -> i64 {
    platform_offset_secs(secs)
}

// UTC [year, month, day, hour, minute, second] for unix seconds (civil-from-days).
pub fn utc_civil(secs: i64) -> [i64; 6] {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let rem = secs.rem_euclid(86_400);
    [y, m, d, rem / 3600, rem % 3600 / 60, rem % 60]
}

// Unix seconds of a civil [year, month, day, hour, minute, second] read as UTC (inverse of utc_civil).
fn civil_secs(c: [i64; 6]) -> i64 {
    let [y, mo, d, h, mi, s] = c;
    let y = if mo <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if mo > 2 { mo - 3 } else { mo + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    days * 86_400 + h * 3600 + mi * 60 + s
}

#[cfg(unix)]
fn platform_offset_secs(secs: i64) -> i64 {
    let t = secs as libc::time_t;
    // SAFETY: localtime_r writes into the zeroed tm we own and reads only the local `t`.
    let tm = unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        tm
    };
    let local = [
        tm.tm_year as i64 + 1900,
        tm.tm_mon as i64 + 1,
        tm.tm_mday as i64,
        tm.tm_hour as i64,
        tm.tm_min as i64,
        tm.tm_sec as i64,
    ];
    civil_secs(local) - secs
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[cfg(windows)]
unsafe extern "system" {
    #[link_name = "SystemTimeToTzSpecificLocalTime"]
    fn system_time_to_tz_local(
        tz: *const std::ffi::c_void,
        utc: *const SystemTime,
        local: *mut SystemTime,
    ) -> i32;
}

#[cfg(windows)]
fn platform_offset_secs(secs: i64) -> i64 {
    let [y, mo, d, h, mi, s] = utc_civil(secs);
    let utc = SystemTime {
        year: y as u16,
        month: mo as u16,
        day: d as u16,
        hour: h as u16,
        minute: mi as u16,
        second: s as u16,
        ..SystemTime::default()
    };
    let mut local = SystemTime::default();
    // SAFETY: both pointers reference live SYSTEMTIME values; a null zone means the current local zone.
    let ok = unsafe { system_time_to_tz_local(std::ptr::null(), &utc, &mut local) };
    if ok == 0 {
        return 0;
    }
    let local = [
        i64::from(local.year),
        i64::from(local.month),
        i64::from(local.day),
        i64::from(local.hour),
        i64::from(local.minute),
        i64::from(local.second),
    ];
    civil_secs(local) - secs
}

// Other targets: no OS zone source wired up, so UTC keeps the stamps well-formed.
#[cfg(not(any(unix, windows)))]
fn platform_offset_secs(_secs: i64) -> i64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_civil_matches_known_epoch() {
        // 1700000000 = 2023-11-14 22:13:20 UTC.
        assert_eq!(utc_civil(1_700_000_000), [2023, 11, 14, 22, 13, 20]);
        assert_eq!(utc_civil(0), [1970, 1, 1, 0, 0, 0]);
    }

    #[test]
    fn civil_secs_inverts_utc_civil() {
        for secs in [0, 951_782_400, 1_700_000_000, 4_102_444_799, -86_401] {
            assert_eq!(civil_secs(utc_civil(secs)), secs);
        }
    }

    #[test]
    fn offset_formats_kst_and_negative_zones() {
        // 1700000000 seen at UTC+9 (KST) is 2023-11-15 07:13:20; at UTC-5 it is 2023-11-14 17:13:20.
        assert_eq!(
            offset_civil(1_700_000_000, 9 * 3600),
            [2023, 11, 15, 7, 13, 20]
        );
        assert_eq!(
            offset_civil(1_700_000_000, -5 * 3600),
            [2023, 11, 14, 17, 13, 20]
        );
        assert_eq!(offset_civil(1_700_000_000, 0), utc_civil(1_700_000_000));
    }

    #[test]
    fn local_civil_is_offset_applied_to_utc() {
        let secs = 1_700_000_000;
        assert_eq!(
            local_civil(secs),
            offset_civil(secs, local_offset_secs(secs))
        );
        let off = local_offset_secs(secs);
        assert!((-14 * 3600..=14 * 3600).contains(&off));
    }
}
