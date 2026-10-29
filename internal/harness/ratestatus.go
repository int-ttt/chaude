package harness

import (
	"fmt"
	"math"
	"strconv"
	"time"
)

// fmtRateStatus: "now/reset · N%" from the five-hour rate window; "" when no percent is known.
func fmtRateStatus(info map[string]any, now time.Time) string {
	w := asMap(asMap(info["unifiedWindows"])["five_hour"])
	v, ok := rateNum(w["utilization"])
	top := str(info["rateLimitType"]) // top-level fields only count for the 5h window
	if !ok && (top == "" || top == "five_hour") {
		v, ok = rateNum(info["utilization"])
	}
	if !ok {
		return ""
	}
	if v < 1 { // fraction scale (0.37 -> 37%)
		v *= 100
	}
	reset := now
	rv, ok := rateNum(w["resetsAt"])
	if !ok && (top == "" || top == "five_hour") {
		rv, ok = rateNum(info["resetsAt"])
	}
	if ok {
		if rv > 1e12 {
			rv /= 1000
		}
		reset = time.Unix(int64(rv), 0)
	}
	return fmt.Sprintf("%s/%s · %d%%", now.Format("15:04"), reset.Format("15:04"), int(math.Round(v)))
}

func rateNum(v any) (float64, bool) {
	switch x := v.(type) {
	case float64:
		return x, true
	case int:
		return float64(x), true
	case string:
		f, err := strconv.ParseFloat(x, 64)
		return f, err == nil
	}
	return 0, false
}
