package harness

import (
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"
)

// DefaultMaxWorkers is the default cap on concurrent live worker sessions.
const DefaultMaxWorkers = 3

// resolveMaxWorkers picks the worker cap: the -max-workers flag when set, else the
// CHAUDE_MAX_WORKERS env value when non-empty, else DefaultMaxWorkers. Values below 1 are errors.
func resolveMaxWorkers(flagSet bool, flagVal int, env string) (int, error) {
	if flagSet {
		if flagVal < 1 {
			return 0, fmt.Errorf("-max-workers must be >= 1 (got %d)", flagVal)
		}
		return flagVal, nil
	}
	if s := strings.TrimSpace(env); s != "" {
		n, err := strconv.Atoi(s)
		if err != nil || n < 1 {
			return 0, fmt.Errorf("CHAUDE_MAX_WORKERS must be an integer >= 1 (got %q)", env)
		}
		return n, nil
	}
	return DefaultMaxWorkers, nil
}

// checkWorkerCap refuses a new live worker when set already holds max children.
// Existing children are left running.
func checkWorkerCap(set *childSet, max int) error {
	if set == nil {
		return nil
	}
	set.mu.Lock()
	n := len(set.m)
	set.mu.Unlock()
	if n >= max {
		return fmt.Errorf("워커 세션 상한(%d개)에 도달해 새 세션을 시작하지 않습니다. 기존 세션을 정리하거나 -max-workers 값을 올려 주세요.", max)
	}
	return nil
}

// childAction is what dispatch should do to reach a (cwd, model) child.
type childAction int

const (
	actStartNew childAction = iota // no warm child anywhere: start a fresh claude -p
	actUseLive                     // a parked-live child in this process is still warm: reuse it
	actResume                      // a parked registry row is still warm: resume its session id
)

// pickChild decides how to reach the child for key (childKey(cwd, model)).
// A live child in set wins when isWarm. Otherwise a registry row for (cwd, model) that is parked,
// warm, and not owned by another live harness process is resumed. Everything else starts new.
// A nil set or nil reg means that source is absent.
func pickChild(set *childSet, reg *clientRegistry, key, cwd, model string, limit int, now time.Time, deadline time.Duration) childAction {
	if set != nil {
		set.mu.Lock()
		c, ok := set.m[key]
		set.mu.Unlock()
		if ok && isWarm(clientRec{State: clientLive, LastAPI: c.lastAPI(), LastCtx: c.LastCtx}, limit, now, deadline) {
			return actUseLive
		}
	}
	rec, ok := reg.Get(cwd, model)
	if !ok || rec.State != clientParked || rec.SessionID == "" {
		return actStartNew
	}
	if rec.OwnerPID != 0 && rec.OwnerPID != os.Getpid() && pidAlive(rec.OwnerPID) {
		return actStartNew // another running chaude owns this row
	}
	if !isWarm(rec, limit, now, deadline) {
		return actStartNew
	}
	return actResume
}
