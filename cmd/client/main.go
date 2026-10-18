package main

import (
	"fmt"
	"os"
	"packy/internal/client/rpc"
)

func main() {
	if e := rpc.Run(); e != nil {
		fmt.Fprintln(os.Stderr, e)
		os.Exit(1)
	}
}
