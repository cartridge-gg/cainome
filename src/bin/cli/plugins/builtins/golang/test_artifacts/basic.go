// Code generated by Cainome. DO NOT EDIT.
// Generated from ABI file.

package abigen

import (
	"context"
	"fmt"
	"github.com/NethermindEth/juno/core/felt"
	"github.com/NethermindEth/starknet.go/account"
	"github.com/NethermindEth/starknet.go/rpc"
	"github.com/NethermindEth/starknet.go/utils"
	"github.com/cartridge-gg/cainome"
	"math/big"
)

// BasicBasicEvent represents a contract event
type BasicBasicEvent interface {
	IsBasicBasicEvent() bool
}


type BasicContract struct {
	contractAddress *felt.Felt
}

func NewBasicContract(contractAddress *felt.Felt) *BasicContract {
	return &BasicContract {
		contractAddress: contractAddress,
	}
}

type BasicReader struct {
	*BasicContract
	provider rpc.RpcProvider
}

type BasicWriter struct {
	*BasicContract
	account *account.Account
}

type Basic struct {
	*BasicReader
	*BasicWriter
}

func NewBasicReader(contractAddress *felt.Felt, provider rpc.RpcProvider) *BasicReader {
	return &BasicReader {
		BasicContract: NewBasicContract(contractAddress),
		provider: provider,
	}
}

func NewBasicWriter(contractAddress *felt.Felt, account *account.Account) *BasicWriter {
	return &BasicWriter {
		BasicContract: NewBasicContract(contractAddress),
		account: account,
	}
}

func NewBasic(contractAddress *felt.Felt, account *account.Account) *Basic {
	return &Basic {
		BasicReader: NewBasicReader(contractAddress, account.Provider),
		BasicWriter: NewBasicWriter(contractAddress, account),
	}
}

type BasicReadStorageTupleResponse struct {
	Value struct {
	Field0 *felt.Felt
	Field1 *big.Int
} `json:"value"`
}

func NewBasicReadStorageTupleResponse(value struct {
	Field0 *felt.Felt
	Field1 *big.Int
}) *BasicReadStorageTupleResponse {
	return &BasicReadStorageTupleResponse {
		Value: value,
	}
}

// MarshalCairo serializes BasicReadStorageTupleResponse to Cairo felt array
func (s *BasicReadStorageTupleResponse) MarshalCairo() ([]*felt.Felt, error) {
	var result []*felt.Felt

	// Tuple field Value: marshal each sub-field (tuple has 2 elements)
	result = append(result, s.Value.Field0)
	result = append(result, cainome.FeltFromBigInt(s.Value.Field1))

	return result, nil
}

// UnmarshalCairo deserializes BasicReadStorageTupleResponse from Cairo felt array
func (s *BasicReadStorageTupleResponse) UnmarshalCairo(data []*felt.Felt) error {
	offset := 0

	// Tuple field Value: unmarshal each sub-field
	if offset >= len(data) {
		return fmt.Errorf("insufficient data for tuple field Value element 0")
	}
	s.Value.Field0 = data[offset]
	offset++
	if offset >= len(data) {
		return fmt.Errorf("insufficient data for tuple field Value element 1")
	}
	s.Value.Field1 = cainome.BigIntFromFelt(data[offset])
	offset++


	return nil
}

// CairoSize returns the serialized size for BasicReadStorageTupleResponse
func (s *BasicReadStorageTupleResponse) CairoSize() int {
	return -1 // Dynamic size
}

type BasicSetStorageInput struct {
	V1 *felt.Felt `json:"v_1"`
	V2 *big.Int `json:"v_2"`
}

func NewBasicSetStorageInput(v_1 *felt.Felt, v_2 *big.Int) *BasicSetStorageInput {
	return &BasicSetStorageInput {
		V1: v_1,
		V2: v_2,
	}
}

// MarshalCairo serializes BasicSetStorageInput to Cairo felt array
func (s *BasicSetStorageInput) MarshalCairo() ([]*felt.Felt, error) {
	var result []*felt.Felt

	result = append(result, s.V1)
	result = append(result, cainome.FeltFromBigInt(s.V2))

	return result, nil
}

// UnmarshalCairo deserializes BasicSetStorageInput from Cairo felt array
func (s *BasicSetStorageInput) UnmarshalCairo(data []*felt.Felt) error {
	offset := 0

	if offset >= len(data) {
		return fmt.Errorf("insufficient data for field V1")
	}
	s.V1 = data[offset]
	offset++

	if offset >= len(data) {
		return fmt.Errorf("insufficient data for field V2")
	}
	s.V2 = cainome.BigIntFromFelt(data[offset])
	offset++


	return nil
}

// CairoSize returns the serialized size for BasicSetStorageInput
func (s *BasicSetStorageInput) CairoSize() int {
	return -1 // Dynamic size
}

type BasicSetStorageResponse struct {
	// This function has no return values
}

func NewBasicSetStorageResponse() *BasicSetStorageResponse {
	return &BasicSetStorageResponse{}
}

// MarshalCairo serializes BasicSetStorageResponse to Cairo felt array
func (s *BasicSetStorageResponse) MarshalCairo() ([]*felt.Felt, error) {
	var result []*felt.Felt


	return result, nil
}

// UnmarshalCairo deserializes BasicSetStorageResponse from Cairo felt array
func (s *BasicSetStorageResponse) UnmarshalCairo(data []*felt.Felt) error {

	return nil
}

// CairoSize returns the serialized size for BasicSetStorageResponse
func (s *BasicSetStorageResponse) CairoSize() int {
	return -1 // Dynamic size
}

func (basic_contract *BasicContract) ReadStorageTuple() (rpc.FunctionCall, error) {
	// Serialize input to calldata
	calldata := []*felt.Felt{}

	return rpc.FunctionCall{
		ContractAddress:    basic_contract.contractAddress,
		EntryPointSelector: utils.GetSelectorFromNameFelt("readStorageTuple"),
		Calldata:           calldata,
	}, nil
}

func (basic_contract *BasicContract) ReadStorageTupleLegacy() (rpc.FunctionCall, error) {
	// Serialize parameters to calldata
	calldata := []*felt.Felt{}

	return rpc.FunctionCall{
		ContractAddress:    basic_contract.contractAddress,
		EntryPointSelector: utils.GetSelectorFromNameFelt("readStorageTuple"),
		Calldata:           calldata,
	}, nil
}

func (basic_contract *BasicContract) SetStorage(input *BasicSetStorageInput) (rpc.FunctionCall, error) {
	// Serialize input to calldata
	calldata, err := input.MarshalCairo()
	if err != nil {
		return rpc.FunctionCall{}, err
	}

	return rpc.FunctionCall{
		ContractAddress:    basic_contract.contractAddress,
		EntryPointSelector: utils.GetSelectorFromNameFelt("setStorage"),
		Calldata:           calldata,
	}, nil
}

func (basic_contract *BasicContract) SetStorageLegacy(v_1 *felt.Felt, v_2 *big.Int) (rpc.FunctionCall, error) {
	// Serialize parameters to calldata
	calldata := []*felt.Felt{}
	calldata = append(calldata, v_1)
	calldata = append(calldata, cainome.FeltFromBigInt(v_2))

	return rpc.FunctionCall{
		ContractAddress:    basic_contract.contractAddress,
		EntryPointSelector: utils.GetSelectorFromNameFelt("setStorage"),
		Calldata:           calldata,
	}, nil
}

func (basic_reader *BasicReader) ReadStorageTuple(ctx context.Context, opts *cainome.CallOpts) (struct {
	Field0 *felt.Felt
	Field1 *big.Int
}, error) {
	// Setup call options
	if opts == nil {
		opts = &cainome.CallOpts{}
	}
	var blockID rpc.BlockID
	if opts.BlockID != nil {
		blockID = *opts.BlockID
	} else {
		blockID = rpc.BlockID{Tag: "latest"}
	}

	// No parameters required
	calldata := []*felt.Felt{}

	// Make the contract call
	functionCall := rpc.FunctionCall{
		ContractAddress:    basic_reader.contractAddress,
		EntryPointSelector: utils.GetSelectorFromNameFelt("readStorageTuple"),
		Calldata:           calldata,
	}

	response, err := basic_reader.provider.Call(ctx, functionCall, blockID)
	if err != nil {
		return struct {
	Field0 *felt.Felt
	Field1 *big.Int
}{}, err
	}

	// Deserialize response to proper type
	if len(response) == 0 {
		return struct {
	Field0 *felt.Felt
	Field1 *big.Int
}{}, fmt.Errorf("empty response")
	}
	var result struct {
	Field0 *felt.Felt
	Field1 *big.Int
}
	offset := 0

	if offset >= len(response) {
		return struct {
	Field0 *felt.Felt
	Field1 *big.Int
}{}, fmt.Errorf("insufficient data for tuple field 0")
	}
	result.Field0 = response[offset]
	offset++

	if offset >= len(response) {
		return struct {
	Field0 *felt.Felt
	Field1 *big.Int
}{}, fmt.Errorf("insufficient data for tuple field 1")
	}
	result.Field1 = cainome.BigIntFromFelt(response[offset])
	offset++

	return result, nil
}

func (basic_writer *BasicWriter) SetStorage(ctx context.Context, v_1 *felt.Felt, v_2 *big.Int, opts *cainome.InvokeOpts) (*felt.Felt, error) {
	// Setup invoke options
	if opts == nil {
		opts = &cainome.InvokeOpts{}
	}

	// Serialize parameters to calldata
	calldata := []*felt.Felt{}
	calldata = append(calldata, v_1)
	calldata = append(calldata, cainome.FeltFromBigInt(v_2))

	// Build and send invoke transaction using cainome helper
	txHash, err := cainome.BuildAndSendInvokeTxn(ctx, basic_writer.account, basic_writer.contractAddress, utils.GetSelectorFromNameFelt("setStorage"), calldata, opts)
	if err != nil {
		return nil, fmt.Errorf("failed to submit invoke transaction: %w", err)
	}

	return txHash, nil
}

