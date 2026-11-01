package scripts

import (
	"compress/gzip"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

func init() { register("compress-archive", compressArchive) }

func compressArchive(args []string) int {
	flags := newFS("compress-archive")
	dirs, rc := parseArgs(flags, args)
	if rc >= 0 {
		return rc
	}
	if len(dirs) == 0 {
		flags.Usage()
		return 2
	}
	var before, after int64
	n := 0
	for _, d := range dirs {
		var files []string
		_ = filepath.WalkDir(d, func(p string, de fs.DirEntry, err error) error {
			if err != nil {
				return nil
			}
			if p != d && strings.HasPrefix(de.Name(), ".") {
				if de.IsDir() {
					return filepath.SkipDir
				}
				return nil
			}
			if !de.IsDir() && strings.HasSuffix(p, ".jsonl") {
				files = append(files, p)
			}
			return nil
		})
		for _, p := range files {
			if err := gzipFile(p); err != nil {
				return fail("%v", err)
			}
			a, _ := os.Stat(p)
			b, _ := os.Stat(p + ".gz")
			before += a.Size()
			after += b.Size()
			os.Remove(p)
			n++
		}
	}
	if n > 0 {
		fmt.Printf("   압축: 기록 %d개 %.1fMB → %.1fMB\n", n, float64(before)/1e6, float64(after)/1e6)
	} else {
		fmt.Printf("   압축할 기록 없음: %s\n", strings.Join(dirs, ", "))
	}
	return 0
}

func gzipFile(p string) error {
	src, err := os.Open(p)
	if err != nil {
		return err
	}
	defer src.Close()
	dst, err := os.Create(p + ".gz")
	if err != nil {
		return err
	}
	zw, _ := gzip.NewWriterLevel(dst, 6)
	if _, err = io.Copy(zw, src); err != nil {
		dst.Close()
		return err
	}
	if err = zw.Close(); err != nil {
		dst.Close()
		return err
	}
	return dst.Close()
}
