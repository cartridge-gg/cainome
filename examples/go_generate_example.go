// Package main demonstrates how to use cainome with go generate
//
// To regenerate the bindings, run:
//   go generate ./...
//
// This will execute the cainome tool and generate Go bindings for the specified contract.

//go:generate go run -mod=mod github.com/cartridge-gg/cainome/src/bin/cainome-go --golang --golang-package mycontract --output-dir ./generated --execution-version v3 --artifacts-path ../contracts/target/dev

package main

import (
	"context"
	"fmt"
	"log"

	"github.com/NethermindEth/starknet.go/rpc"
	"github.com/cartridge-gg/cainome/examples/generated/mycontract"
)

func main() {
	// Example usage of generated bindings
	// Note: This is just an example structure - actual usage depends on your contract

	// Create a StarkNet provider
	provider, err := rpc.NewProvider("https://starknet-mainnet.public.blastapi.io")
	if err != nil {
		log.Fatal(err)
	}

	// Contract address (replace with your actual contract address)
	contractAddress := "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"

	// Create contract reader for view functions
	reader := mycontract.NewReader(contractAddress, provider)

	// Call a view function (example)
	ctx := context.Background()
	result, err := reader.GetValue(ctx)
	if err != nil {
		log.Printf("Error calling view function: %v", err)
		return
	}

	fmt.Printf("Contract value: %v\n", result)

	// For write operations, you would need an account:
	// writer := mycontract.NewWriter(contractAddress, account)
	// txResult, err := writer.SetValue(ctx, newValue)
}
