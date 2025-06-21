// Package cainome provides Cairo contract ABI bindings and type-safe interaction utilities for Go applications.
//
// This package includes:
// - CairoMarshaler interface for serializing Go types to/from Cairo felt arrays
// - All Cairo primitive types (u8, u16, u32, u64, u128, u256, felt, bool)
// - StarkNet types (ContractAddress, ClassHash, EthAddress)
// - Generic Result and Option types
// - Array and tuple support
// - Helper functions for type conversion
//
// Example usage:
//   import "github.com/cartridge-gg/cainome"
//
//   // Create a Cairo-compatible felt value
//   value := cainome.NewCairoFelt(new(felt.Felt).SetUint64(123))
//   data, err := value.MarshalCairo()
//
//   // Create Result types
//   result := cainome.NewResultOk[uint64, string](42)
//
//   // Create StarkNet addresses
//   addr := cainome.NewContractAddress(new(felt.Felt).SetUint64(123))
package cainome

import (
	"fmt"
	"math/big"

	"github.com/NethermindEth/juno/core/felt"
	"github.com/NethermindEth/starknet.go/rpc"
)

// CairoMarshaler is the interface for types that can be serialized to/from Cairo format
type CairoMarshaler interface {
	MarshalCairo() ([]*felt.Felt, error)
	UnmarshalCairo(data []*felt.Felt) error
}

// CairoSerde provides serialization helpers with size information
type CairoSerde interface {
	CairoMarshaler
	CairoSize() int // -1 for dynamic size, positive number for fixed size
}

// ============================================================================
// Call configuration types for contract interaction
// ============================================================================

// CallOpts contains options for contract view calls
type CallOpts struct {
	BlockID *rpc.BlockID // Optional block ID (defaults to "latest" if nil)
}

// CallOption defines a function type for setting call options
type CallOption func(*CallOpts)

// WithBlockID sets the block ID for the call
func WithBlockID(blockID rpc.BlockID) CallOption {
	return func(opts *CallOpts) {
		opts.BlockID = &blockID
	}
}

// NewCallOpts creates a new CallOpts with optional configurations
func NewCallOpts(options ...CallOption) *CallOpts {
	opts := &CallOpts{}
	for _, option := range options {
		option(opts)
	}
	return opts
}

// ============================================================================
// Helper functions for type conversion between Go types and Cairo felt values
// ============================================================================

// FeltFromUint converts uint64 to *felt.Felt
func FeltFromUint(value uint64) *felt.Felt {
	return new(felt.Felt).SetUint64(value)
}

// UintFromFelt converts *felt.Felt to uint64
func UintFromFelt(f *felt.Felt) uint64 {
	if f == nil {
		return 0
	}
	bigInt := f.BigInt(big.NewInt(0))
	if !bigInt.IsUint64() {
		return 0 // or handle overflow differently
	}
	return bigInt.Uint64()
}

// FeltFromInt converts int64 to *felt.Felt
func FeltFromInt(value int64) *felt.Felt {
	// Cairo/StarkNet uses field arithmetic, so negative numbers are represented
	// as positive values in the field. For negative values, we use two's complement.
	if value < 0 {
		// Convert to field element using modular arithmetic
		// The field modulus is 2^251 + 17 * 2^192 + 1, but felt.Felt handles this internally
		bigInt := big.NewInt(value)
		return FeltFromBigInt(bigInt)
	}
	return new(felt.Felt).SetUint64(uint64(value))
}

// IntFromFelt converts *felt.Felt to int64
func IntFromFelt(f *felt.Felt) int64 {
	if f == nil {
		return 0
	}
	// Handle potential overflow from felt to int64
	bigInt := f.BigInt(big.NewInt(0))
	if !bigInt.IsInt64() {
		return 0 // or handle overflow differently
	}
	return bigInt.Int64()
}

// FeltFromBigInt converts *big.Int to *felt.Felt
func FeltFromBigInt(value *big.Int) *felt.Felt {
	if value == nil {
		return new(felt.Felt)
	}
	f := new(felt.Felt)
	f.SetBytes(value.Bytes())
	return f
}

// BigIntFromFelt converts *felt.Felt to *big.Int
func BigIntFromFelt(f *felt.Felt) *big.Int {
	if f == nil {
		return big.NewInt(0)
	}
	return f.BigInt(big.NewInt(0))
}

// FeltFromBool converts bool to *felt.Felt
func FeltFromBool(value bool) *felt.Felt {
	if value {
		return FeltFromUint(1)
	}
	return FeltFromUint(0)
}

// BoolFromFelt converts *felt.Felt to bool
func BoolFromFelt(f *felt.Felt) bool {
	return UintFromFelt(f) != 0
}

// FeltFromBytes converts byte slice to *felt.Felt
func FeltFromBytes(data []byte) *felt.Felt {
	if len(data) == 0 {
		return new(felt.Felt)
	}
	// Ensure we don't exceed felt size (252 bits = 31.5 bytes)
	if len(data) > 31 {
		data = data[:31]
	}
	f := new(felt.Felt)
	f.SetBytes(data)
	return f
}

// BytesFromFelt converts *felt.Felt to byte slice
func BytesFromFelt(f *felt.Felt) []byte {
	if f == nil {
		return []byte{}
	}
	bytes := f.Bytes()
	return bytes[:]
}

// FeltFromString converts string to *felt.Felt using UTF-8 encoding
func FeltFromString(s string) *felt.Felt {
	return FeltFromBytes([]byte(s))
}

// StringFromFelt converts *felt.Felt to string using UTF-8 decoding
func StringFromFelt(f *felt.Felt) string {
	return string(BytesFromFelt(f))
}

// ============================================================================
// Basic Cairo type wrappers that implement CairoMarshaler
// ============================================================================

// CairoFelt wraps *felt.Felt with CairoMarshaler implementation
type CairoFelt struct {
	Value *felt.Felt
}

func NewCairoFelt(value *felt.Felt) *CairoFelt {
	return &CairoFelt{Value: value}
}

func (f *CairoFelt) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{f.Value}, nil
}

func (f *CairoFelt) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for felt")
	}
	f.Value = data[0]
	return nil
}

func (f *CairoFelt) CairoSize() int {
	return 1
}

// CairoBool wraps bool with CairoMarshaler implementation
type CairoBool struct {
	Value bool
}

func NewCairoBool(value bool) *CairoBool {
	return &CairoBool{Value: value}
}

func (b *CairoBool) MarshalCairo() ([]*felt.Felt, error) {
	if b.Value {
		return []*felt.Felt{FeltFromUint(1)}, nil
	}
	return []*felt.Felt{FeltFromUint(0)}, nil
}

func (b *CairoBool) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for bool")
	}
	b.Value = UintFromFelt(data[0]) != 0
	return nil
}

func (b *CairoBool) CairoSize() int {
	return 1
}

// CairoUint8 wraps uint8 with CairoMarshaler implementation
type CairoUint8 struct {
	Value uint8
}

func NewCairoUint8(value uint8) *CairoUint8 {
	return &CairoUint8{Value: value}
}

func (u *CairoUint8) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromUint(uint64(u.Value))}, nil
}

func (u *CairoUint8) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for uint8")
	}
	u.Value = uint8(UintFromFelt(data[0]))
	return nil
}

func (u *CairoUint8) CairoSize() int {
	return 1
}

// CairoUint16 wraps uint16 with CairoMarshaler implementation
type CairoUint16 struct {
	Value uint16
}

func NewCairoUint16(value uint16) *CairoUint16 {
	return &CairoUint16{Value: value}
}

func (u *CairoUint16) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromUint(uint64(u.Value))}, nil
}

func (u *CairoUint16) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for uint16")
	}
	u.Value = uint16(UintFromFelt(data[0]))
	return nil
}

func (u *CairoUint16) CairoSize() int {
	return 1
}

// CairoUint32 wraps uint32 with CairoMarshaler implementation
type CairoUint32 struct {
	Value uint32
}

func NewCairoUint32(value uint32) *CairoUint32 {
	return &CairoUint32{Value: value}
}

func (u *CairoUint32) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromUint(uint64(u.Value))}, nil
}

func (u *CairoUint32) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for uint32")
	}
	u.Value = uint32(UintFromFelt(data[0]))
	return nil
}

func (u *CairoUint32) CairoSize() int {
	return 1
}

// CairoUint64 wraps uint64 with CairoMarshaler implementation
type CairoUint64 struct {
	Value uint64
}

func NewCairoUint64(value uint64) *CairoUint64 {
	return &CairoUint64{Value: value}
}

func (u *CairoUint64) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromUint(u.Value)}, nil
}

func (u *CairoUint64) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for uint64")
	}
	u.Value = UintFromFelt(data[0])
	return nil
}

func (u *CairoUint64) CairoSize() int {
	return 1
}

// CairoUint128 wraps big.Int for 128-bit unsigned integers
type CairoUint128 struct {
	Value *big.Int
}

func NewCairoUint128(value *big.Int) *CairoUint128 {
	return &CairoUint128{Value: value}
}

func NewCairoUint128FromUint64(value uint64) *CairoUint128 {
	return &CairoUint128{Value: new(big.Int).SetUint64(value)}
}

func (u *CairoUint128) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromBigInt(u.Value)}, nil
}

func (u *CairoUint128) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for uint128")
	}
	u.Value = BigIntFromFelt(data[0])
	return nil
}

func (u *CairoUint128) CairoSize() int {
	return 1
}

// CairoUint256 represents a 256-bit unsigned integer
type CairoUint256 struct {
	Low  *big.Int // Lower 128 bits
	High *big.Int // Upper 128 bits
}

func NewCairoUint256(low, high *big.Int) *CairoUint256 {
	return &CairoUint256{Low: low, High: high}
}

func NewCairoUint256FromUint64(value uint64) *CairoUint256 {
	return &CairoUint256{
		Low:  new(big.Int).SetUint64(value),
		High: new(big.Int),
	}
}

func NewCairoUint256FromBigInt(value *big.Int) *CairoUint256 {
	// Split big.Int into low and high 128-bit parts
	mask := new(big.Int).Lsh(big.NewInt(1), 128)
	mask.Sub(mask, big.NewInt(1)) // mask = 2^128 - 1

	low := new(big.Int).And(value, mask)
	high := new(big.Int).Rsh(value, 128)

	return &CairoUint256{Low: low, High: high}
}

func (u *CairoUint256) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromBigInt(u.Low), FeltFromBigInt(u.High)}, nil
}

func (u *CairoUint256) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) < 2 {
		return fmt.Errorf("insufficient data for uint256: need 2 felts, got %d", len(data))
	}
	u.Low = BigIntFromFelt(data[0])
	u.High = BigIntFromFelt(data[1])
	return nil
}

func (u *CairoUint256) CairoSize() int {
	return 2
}

// ToBigInt converts CairoUint256 to a single *big.Int
func (u *CairoUint256) ToBigInt() *big.Int {
	result := new(big.Int).Set(u.High)
	result.Lsh(result, 128)
	result.Add(result, u.Low)
	return result
}

// ============================================================================
// StarkNet-specific types
// ============================================================================

// ContractAddress represents a StarkNet contract address
type ContractAddress struct {
	Value *felt.Felt
}

func NewContractAddress(value *felt.Felt) *ContractAddress {
	return &ContractAddress{Value: value}
}

func (a *ContractAddress) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{a.Value}, nil
}

func (a *ContractAddress) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for ContractAddress")
	}
	a.Value = data[0]
	return nil
}

func (a *ContractAddress) CairoSize() int {
	return 1
}

// ClassHash represents a StarkNet class hash
type ClassHash struct {
	Value *felt.Felt
}

func NewClassHash(value *felt.Felt) *ClassHash {
	return &ClassHash{Value: value}
}

func (h *ClassHash) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{h.Value}, nil
}

func (h *ClassHash) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for ClassHash")
	}
	h.Value = data[0]
	return nil
}

func (h *ClassHash) CairoSize() int {
	return 1
}

// EthAddress represents an Ethereum address in StarkNet context
type EthAddress struct {
	Value *felt.Felt
}

func NewEthAddress(value *felt.Felt) *EthAddress {
	return &EthAddress{Value: value}
}

func (e *EthAddress) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{e.Value}, nil
}

func (e *EthAddress) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for EthAddress")
	}
	e.Value = data[0]
	return nil
}

func (e *EthAddress) CairoSize() int {
	return 1
}

// ============================================================================
// Array types
// ============================================================================

// CairoFeltArray wraps []*felt.Felt with CairoMarshaler implementation
type CairoFeltArray struct {
	Value []*felt.Felt
}

func NewCairoFeltArray(value []*felt.Felt) *CairoFeltArray {
	return &CairoFeltArray{Value: value}
}

func (a *CairoFeltArray) MarshalCairo() ([]*felt.Felt, error) {
	var result []*felt.Felt
	// Array serialization: length first, then elements
	result = append(result, FeltFromUint(uint64(len(a.Value))))
	result = append(result, a.Value...)
	return result, nil
}

func (a *CairoFeltArray) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for array length")
	}

	length := UintFromFelt(data[0])
	if uint64(len(data)) < length+1 {
		return fmt.Errorf("insufficient data for array elements: expected %d, got %d", length+1, len(data))
	}

	a.Value = make([]*felt.Felt, length)
	for i := uint64(0); i < length; i++ {
		a.Value[i] = data[1+i]
	}
	return nil
}

func (a *CairoFeltArray) CairoSize() int {
	return -1 // Dynamic size
}

// ============================================================================
// Generic Result type for Cairo Result<T, E>
// ============================================================================

type Result[T, E any] struct {
	IsOk bool
	Ok   T
	Err  E
}

// NewResultOk creates a Result with Ok value
func NewResultOk[T, E any](value T) Result[T, E] {
	var zero E
	return Result[T, E]{
		IsOk: true,
		Ok:   value,
		Err:  zero,
	}
}

// NewResultErr creates a Result with Err value
func NewResultErr[T, E any](err E) Result[T, E] {
	var zero T
	return Result[T, E]{
		IsOk: false,
		Ok:   zero,
		Err:  err,
	}
}

// MarshalCairo serializes Result[T, E] to Cairo felt array
func (r *Result[T, E]) MarshalCairo() ([]*felt.Felt, error) {
	var result []*felt.Felt

	if r.IsOk {
		// Discriminant 0 for Ok
		result = append(result, FeltFromUint(0))

		// Serialize Ok value if it implements CairoMarshaler
		if marshaler, ok := any(r.Ok).(CairoMarshaler); ok {
			data, err := marshaler.MarshalCairo()
			if err != nil {
				return nil, fmt.Errorf("failed to marshal Ok value: %w", err)
			}
			result = append(result, data...)
		} else {
			// For basic types, try to convert directly
			if okFelt := tryConvertToFelt(r.Ok); okFelt != nil {
				result = append(result, okFelt)
			} else {
				return nil, fmt.Errorf("Ok value type %T does not implement CairoMarshaler", r.Ok)
			}
		}
	} else {
		// Discriminant 1 for Err
		result = append(result, FeltFromUint(1))

		// Serialize Err value if it implements CairoMarshaler
		if marshaler, ok := any(r.Err).(CairoMarshaler); ok {
			data, err := marshaler.MarshalCairo()
			if err != nil {
				return nil, fmt.Errorf("failed to marshal Err value: %w", err)
			}
			result = append(result, data...)
		} else {
			// For basic types, try to convert directly
			if errFelt := tryConvertToFelt(r.Err); errFelt != nil {
				result = append(result, errFelt)
			} else {
				return nil, fmt.Errorf("Err value type %T does not implement CairoMarshaler", r.Err)
			}
		}
	}

	return result, nil
}

// UnmarshalCairo deserializes Result[T, E] from Cairo felt array
func (r *Result[T, E]) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for Result discriminant")
	}

	discriminant := UintFromFelt(data[0])

	switch discriminant {
	case 0: // Ok variant
		r.IsOk = true

		// Try to unmarshal Ok value if it implements CairoMarshaler
		if unmarshaler, ok := any(&r.Ok).(CairoMarshaler); ok {
			return unmarshaler.UnmarshalCairo(data[1:])
		} else {
			// For basic types, try to convert directly
			if len(data) < 2 {
				return fmt.Errorf("insufficient data for Ok value")
			}
			if err := tryConvertFromFelt(data[1], &r.Ok); err != nil {
				return fmt.Errorf("failed to unmarshal Ok value: %w", err)
			}
		}

	case 1: // Err variant
		r.IsOk = false

		// Try to unmarshal Err value if it implements CairoMarshaler
		if unmarshaler, ok := any(&r.Err).(CairoMarshaler); ok {
			return unmarshaler.UnmarshalCairo(data[1:])
		} else {
			// For basic types, try to convert directly
			if len(data) < 2 {
				return fmt.Errorf("insufficient data for Err value")
			}
			if err := tryConvertFromFelt(data[1], &r.Err); err != nil {
				return fmt.Errorf("failed to unmarshal Err value: %w", err)
			}
		}

	default:
		return fmt.Errorf("unknown Result discriminant: %d", discriminant)
	}

	return nil
}

// CairoSize returns the serialized size for Result[T, E]
func (r *Result[T, E]) CairoSize() int {
	return -1 // Dynamic size
}

// ============================================================================
// Generic Option type for Cairo Option<T>
// ============================================================================

type Option[T any] struct {
	IsSome bool
	Value  T
}

// NewOptionSome creates an Option with a value
func NewOptionSome[T any](value T) Option[T] {
	return Option[T]{
		IsSome: true,
		Value:  value,
	}
}

// NewOptionNone creates an Option with no value
func NewOptionNone[T any]() Option[T] {
	var zero T
	return Option[T]{
		IsSome: false,
		Value:  zero,
	}
}

// MarshalCairo serializes Option[T] to Cairo felt array
func (o *Option[T]) MarshalCairo() ([]*felt.Felt, error) {
	var result []*felt.Felt

	if o.IsSome {
		// Discriminant 0 for Some
		result = append(result, FeltFromUint(0))

		// Serialize value if it implements CairoMarshaler
		if marshaler, ok := any(o.Value).(CairoMarshaler); ok {
			data, err := marshaler.MarshalCairo()
			if err != nil {
				return nil, fmt.Errorf("failed to marshal Some value: %w", err)
			}
			result = append(result, data...)
		} else {
			// For basic types, try to convert directly
			if valueFelt := tryConvertToFelt(o.Value); valueFelt != nil {
				result = append(result, valueFelt)
			} else {
				return nil, fmt.Errorf("Some value type %T does not implement CairoMarshaler", o.Value)
			}
		}
	} else {
		// Discriminant 1 for None
		result = append(result, FeltFromUint(1))
	}

	return result, nil
}

// UnmarshalCairo deserializes Option[T] from Cairo felt array
func (o *Option[T]) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for Option discriminant")
	}

	discriminant := UintFromFelt(data[0])

	switch discriminant {
	case 0: // Some variant
		o.IsSome = true

		// Try to unmarshal value if it implements CairoMarshaler
		if unmarshaler, ok := any(&o.Value).(CairoMarshaler); ok {
			return unmarshaler.UnmarshalCairo(data[1:])
		} else {
			// For basic types, try to convert directly
			if len(data) < 2 {
				return fmt.Errorf("insufficient data for Some value")
			}
			if err := tryConvertFromFelt(data[1], &o.Value); err != nil {
				return fmt.Errorf("failed to unmarshal Some value: %w", err)
			}
		}

	case 1: // None variant
		o.IsSome = false
		var zero T
		o.Value = zero

	default:
		return fmt.Errorf("unknown Option discriminant: %d", discriminant)
	}

	return nil
}

// CairoSize returns the serialized size for Option[T]
func (o *Option[T]) CairoSize() int {
	return -1 // Dynamic size
}

// ============================================================================
// Internal helper functions
// ============================================================================

// tryConvertToFelt attempts to convert common Go types to *felt.Felt
func tryConvertToFelt(value any) *felt.Felt {
	switch v := value.(type) {
	case *felt.Felt:
		return v
	case uint64:
		return FeltFromUint(v)
	case uint32:
		return FeltFromUint(uint64(v))
	case uint16:
		return FeltFromUint(uint64(v))
	case uint8:
		return FeltFromUint(uint64(v))
	case uint:
		return FeltFromUint(uint64(v))
	case int64:
		return FeltFromInt(v)
	case int32:
		return FeltFromInt(int64(v))
	case int16:
		return FeltFromInt(int64(v))
	case int8:
		return FeltFromInt(int64(v))
	case int:
		return FeltFromInt(int64(v))
	case bool:
		return FeltFromBool(v)
	case *big.Int:
		return FeltFromBigInt(v)
	default:
		return nil
	}
}

// tryConvertFromFelt attempts to convert *felt.Felt to common Go types
func tryConvertFromFelt(f *felt.Felt, target any) error {
	switch ptr := target.(type) {
	case **felt.Felt:
		*ptr = f
	case *uint64:
		*ptr = UintFromFelt(f)
	case *uint32:
		*ptr = uint32(UintFromFelt(f))
	case *uint16:
		*ptr = uint16(UintFromFelt(f))
	case *uint8:
		*ptr = uint8(UintFromFelt(f))
	case *uint:
		*ptr = uint(UintFromFelt(f))
	case *int64:
		*ptr = IntFromFelt(f)
	case *int32:
		*ptr = int32(IntFromFelt(f))
	case *int16:
		*ptr = int16(IntFromFelt(f))
	case *int8:
		*ptr = int8(IntFromFelt(f))
	case *int:
		*ptr = int(IntFromFelt(f))
	case *bool:
		*ptr = BoolFromFelt(f)
	case **big.Int:
		*ptr = BigIntFromFelt(f)
	default:
		return fmt.Errorf("unsupported target type %T", target)
	}
	return nil
}

// ============================================================================
// ByteArray support for core::byte_array::ByteArray
// ============================================================================

// CairoByteArray wraps []byte with CairoMarshaler implementation for ByteArray
type CairoByteArray struct {
	Value []byte
}

func NewCairoByteArray(value []byte) *CairoByteArray {
	return &CairoByteArray{Value: value}
}

func (b *CairoByteArray) MarshalCairo() ([]*felt.Felt, error) {
	// ByteArray serialization:
	// 1. Array of bytes31 chunks (each chunk is 31 bytes max)
	// 2. Pending word (felt)
	// 3. Pending word length (u32)
	
	var result []*felt.Felt
	
	// Calculate number of full 31-byte chunks
	fullChunks := len(b.Value) / 31
	remainder := len(b.Value) % 31
	
	// Serialize the array length (number of full chunks)
	result = append(result, FeltFromUint(uint64(fullChunks)))
	
	// Serialize each full 31-byte chunk
	for i := 0; i < fullChunks; i++ {
		chunk := b.Value[i*31 : (i+1)*31]
		// Convert 31 bytes to felt (big-endian)
		result = append(result, FeltFromBytes(chunk))
	}
	
	// Serialize pending word (remaining bytes < 31)
	var pendingWord *felt.Felt
	if remainder > 0 {
		pendingBytes := b.Value[fullChunks*31:]
		pendingWord = FeltFromBytes(pendingBytes)
	} else {
		pendingWord = FeltFromUint(0)
	}
	result = append(result, pendingWord)
	
	// Serialize pending word length
	result = append(result, FeltFromUint(uint64(remainder)))
	
	return result, nil
}

func (b *CairoByteArray) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) < 3 {
		return fmt.Errorf("insufficient data for ByteArray: need at least 3 felts")
	}
	
	offset := 0
	
	// Read array length (number of full chunks)
	numChunks := UintFromFelt(data[offset])
	offset++
	
	// Check we have enough data
	if len(data) < int(1+numChunks+2) {
		return fmt.Errorf("insufficient data for ByteArray: expected %d felts, got %d", 1+numChunks+2, len(data))
	}
	
	var result []byte
	
	// Read each 31-byte chunk
	for i := uint64(0); i < numChunks; i++ {
		chunkBytes := BytesFromFelt(data[offset])
		if len(chunkBytes) > 31 {
			chunkBytes = chunkBytes[len(chunkBytes)-31:] // Take last 31 bytes
		}
		result = append(result, chunkBytes...)
		offset++
	}
	
	// Read pending word
	pendingWord := data[offset]
	offset++
	
	// Read pending word length
	pendingLen := UintFromFelt(data[offset])
	offset++
	
	// Add pending bytes if any
	if pendingLen > 0 {
		pendingBytes := BytesFromFelt(pendingWord)
		if len(pendingBytes) > int(pendingLen) {
			pendingBytes = pendingBytes[len(pendingBytes)-int(pendingLen):] // Take last pendingLen bytes
		}
		result = append(result, pendingBytes...)
	}
	
	b.Value = result
	return nil
}

func (b *CairoByteArray) CairoSize() int {
	return -1 // Dynamic size
}