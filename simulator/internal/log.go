package internal

import (
	"fmt"
	"os"
)

func LogErr(err error, format string, args ...any) {
	msg := fmt.Sprintf(format, args...)
	fmt.Fprintf(os.Stderr, "%s: %v\n", msg, err)
}

func LogInfo(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format, args...)
}
