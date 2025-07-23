package cainome

import (
	"math/big"
	"testing"

	"github.com/NethermindEth/juno/core/felt"
)

// Test helper functions
func TestFeltConversion(t *testing.T) {
	// Test uint conversion
	value := uint64(123)
	f := FeltFromUint(value)
	back := UintFromFelt(f)
	if back != value {
		t.Errorf("FeltFromUint/UintFromFelt roundtrip failed: expected %d, got %d", value, back)
	}

	// Test positive int conversion
	intValue := int64(456)
	f2 := FeltFromInt(intValue)
	back2 := IntFromFelt(f2)
	if back2 != intValue {
		t.Errorf("FeltFromInt/IntFromFelt roundtrip failed: expected %d, got %d", intValue, back2)
	}

	// Test bool conversion
	boolValue := true
	f3 := FeltFromBool(boolValue)
	back3 := BoolFromFelt(f3)
	if back3 != boolValue {
		t.Errorf("FeltFromBool/BoolFromFelt roundtrip failed: expected %v, got %v", boolValue, back3)
	}

	// Test BigInt conversion
	bigValue := new(big.Int).SetInt64(789)
	f4 := FeltFromBigInt(bigValue)
	back4 := BigIntFromFelt(f4)
	if back4.Cmp(bigValue) != 0 {
		t.Errorf("FeltFromBigInt/BigIntFromFelt roundtrip failed: expected %s, got %s", bigValue.String(), back4.String())
	}
}

// Test CairoFelt
func TestCairoFelt(t *testing.T) {
	felt := NewCairoFelt(FeltFromUint(42))
	
	data, err := felt.MarshalCairo()
	if err != nil {
		t.Fatalf("MarshalCairo failed: %v", err)
	}
	if len(data) != 1 {
		t.Errorf("Expected 1 felt, got %d", len(data))
	}
	if UintFromFelt(data[0]) != 42 {
		t.Errorf("Expected 42, got %d", UintFromFelt(data[0]))
	}

	felt2 := &CairoFelt{}
	err = felt2.UnmarshalCairo(data)
	if err != nil {
		t.Fatalf("UnmarshalCairo failed: %v", err)
	}
	if UintFromFelt(felt2.Value) != 42 {
		t.Errorf("Expected 42, got %d", UintFromFelt(felt2.Value))
	}

	if felt.CairoSize() != 1 {
		t.Errorf("Expected size 1, got %d", felt.CairoSize())
	}
}

// Test CairoBool
func TestCairoBool(t *testing.T) {
	tests := []bool{true, false}
	
	for _, test := range tests {
		b := NewCairoBool(test)
		
		data, err := b.MarshalCairo()
		if err != nil {
			t.Fatalf("MarshalCairo failed for %v: %v", test, err)
		}
		if len(data) != 1 {
			t.Errorf("Expected 1 felt, got %d", len(data))
		}
		
		expected := uint64(0)
		if test {
			expected = 1
		}
		if UintFromFelt(data[0]) != expected {
			t.Errorf("Expected %d, got %d", expected, UintFromFelt(data[0]))
		}

		b2 := &CairoBool{}
		err = b2.UnmarshalCairo(data)
		if err != nil {
			t.Fatalf("UnmarshalCairo failed: %v", err)
		}
		if b2.Value != test {
			t.Errorf("Expected %v, got %v", test, b2.Value)
		}
	}
}

// Test all uint types
func TestCairoUints(t *testing.T) {
	// Test uint8
	u8 := NewCairoUint8(255)
	data, err := u8.MarshalCairo()
	if err != nil || len(data) != 1 || UintFromFelt(data[0]) != 255 {
		t.Errorf("CairoUint8 marshal failed")
	}
	u8_2 := &CairoUint8{}
	u8_2.UnmarshalCairo(data)
	if u8_2.Value != 255 {
		t.Errorf("CairoUint8 unmarshal failed")
	}

	// Test uint16
	u16 := NewCairoUint16(65535)
	data, err = u16.MarshalCairo()
	if err != nil || len(data) != 1 || UintFromFelt(data[0]) != 65535 {
		t.Errorf("CairoUint16 marshal failed")
	}
	u16_2 := &CairoUint16{}
	u16_2.UnmarshalCairo(data)
	if u16_2.Value != 65535 {
		t.Errorf("CairoUint16 unmarshal failed")
	}

	// Test uint32
	u32 := NewCairoUint32(4294967295)
	data, err = u32.MarshalCairo()
	if err != nil || len(data) != 1 || UintFromFelt(data[0]) != 4294967295 {
		t.Errorf("CairoUint32 marshal failed")
	}
	u32_2 := &CairoUint32{}
	u32_2.UnmarshalCairo(data)
	if u32_2.Value != 4294967295 {
		t.Errorf("CairoUint32 unmarshal failed")
	}

	// Test uint64
	u64 := NewCairoUint64(18446744073709551615)
	data, err = u64.MarshalCairo()
	if err != nil || len(data) != 1 {
		t.Errorf("CairoUint64 marshal failed")
	}
	u64_2 := &CairoUint64{}
	u64_2.UnmarshalCairo(data)
	if u64_2.Value != 18446744073709551615 {
		t.Errorf("CairoUint64 unmarshal failed")
	}
}

// Test CairoUint128
func TestCairoUint128(t *testing.T) {
	bigVal := new(big.Int).SetUint64(18446744073709551615) // max uint64
	bigVal.Mul(bigVal, big.NewInt(2)) // make it bigger than uint64

	u128 := NewCairoUint128(bigVal)
	
	data, err := u128.MarshalCairo()
	if err != nil {
		t.Fatalf("MarshalCairo failed: %v", err)
	}
	if len(data) != 1 {
		t.Errorf("Expected 1 felt, got %d", len(data))
	}

	u128_2 := &CairoUint128{}
	err = u128_2.UnmarshalCairo(data)
	if err != nil {
		t.Fatalf("UnmarshalCairo failed: %v", err)
	}
	if u128_2.Value.Cmp(bigVal) != 0 {
		t.Errorf("Expected %s, got %s", bigVal.String(), u128_2.Value.String())
	}

	if u128.CairoSize() != 1 {
		t.Errorf("Expected size 1, got %d", u128.CairoSize())
	}
}

// Test CairoUint256
func TestCairoUint256(t *testing.T) {
	low := big.NewInt(123)
	high := big.NewInt(456)
	
	u256 := NewCairoUint256(low, high)
	
	data, err := u256.MarshalCairo()
	if err != nil {
		t.Fatalf("MarshalCairo failed: %v", err)
	}
	if len(data) != 2 {
		t.Errorf("Expected 2 felts, got %d", len(data))
	}

	u256_2 := &CairoUint256{}
	err = u256_2.UnmarshalCairo(data)
	if err != nil {
		t.Fatalf("UnmarshalCairo failed: %v", err)
	}
	if u256_2.Low.Cmp(low) != 0 || u256_2.High.Cmp(high) != 0 {
		t.Errorf("Expected low=%s high=%s, got low=%s high=%s", 
			low.String(), high.String(), u256_2.Low.String(), u256_2.High.String())
	}

	if u256.CairoSize() != 2 {
		t.Errorf("Expected size 2, got %d", u256.CairoSize())
	}

	// Test ToBigInt
	bigInt := u256.ToBigInt()
	expected := new(big.Int).Set(high)
	expected.Lsh(expected, 128)
	expected.Add(expected, low)
	if bigInt.Cmp(expected) != 0 {
		t.Errorf("ToBigInt failed: expected %s, got %s", expected.String(), bigInt.String())
	}

	// Test NewCairoUint256FromBigInt
	u256_3 := NewCairoUint256FromBigInt(expected)
	if u256_3.Low.Cmp(low) != 0 || u256_3.High.Cmp(high) != 0 {
		t.Errorf("NewCairoUint256FromBigInt failed")
	}
}

// Test StarkNet types
func TestStarkNetTypes(t *testing.T) {
	feltValue := FeltFromUint(12345)

	// Test ContractAddress
	addr := NewContractAddress(feltValue)
	data, err := addr.MarshalCairo()
	if err != nil || len(data) != 1 || UintFromFelt(data[0]) != 12345 {
		t.Errorf("ContractAddress marshal failed")
	}
	addr2 := &ContractAddress{}
	addr2.UnmarshalCairo(data)
	if UintFromFelt(addr2.Value) != 12345 {
		t.Errorf("ContractAddress unmarshal failed")
	}

	// Test ClassHash
	hash := NewClassHash(feltValue)
	data, err = hash.MarshalCairo()
	if err != nil || len(data) != 1 || UintFromFelt(data[0]) != 12345 {
		t.Errorf("ClassHash marshal failed")
	}
	hash2 := &ClassHash{}
	hash2.UnmarshalCairo(data)
	if UintFromFelt(hash2.Value) != 12345 {
		t.Errorf("ClassHash unmarshal failed")
	}

	// Test EthAddress
	ethAddr := NewEthAddress(feltValue)
	data, err = ethAddr.MarshalCairo()
	if err != nil || len(data) != 1 || UintFromFelt(data[0]) != 12345 {
		t.Errorf("EthAddress marshal failed")
	}
	ethAddr2 := &EthAddress{}
	ethAddr2.UnmarshalCairo(data)
	if UintFromFelt(ethAddr2.Value) != 12345 {
		t.Errorf("EthAddress unmarshal failed")
	}
}

// Test CairoFeltArray
func TestCairoFeltArray(t *testing.T) {
	felts := []*felt.Felt{FeltFromUint(1), FeltFromUint(2), FeltFromUint(3)}
	array := NewCairoFeltArray(felts)

	data, err := array.MarshalCairo()
	if err != nil {
		t.Fatalf("MarshalCairo failed: %v", err)
	}
	if len(data) != 4 { // length + 3 elements
		t.Errorf("Expected 4 felts, got %d", len(data))
	}
	if UintFromFelt(data[0]) != 3 { // length
		t.Errorf("Expected length 3, got %d", UintFromFelt(data[0]))
	}

	array2 := &CairoFeltArray{}
	err = array2.UnmarshalCairo(data)
	if err != nil {
		t.Fatalf("UnmarshalCairo failed: %v", err)
	}
	if len(array2.Value) != 3 {
		t.Errorf("Expected 3 elements, got %d", len(array2.Value))
	}
	for i, expected := range []uint64{1, 2, 3} {
		if UintFromFelt(array2.Value[i]) != expected {
			t.Errorf("Element %d: expected %d, got %d", i, expected, UintFromFelt(array2.Value[i]))
		}
	}

	if array.CairoSize() != -1 {
		t.Errorf("Expected dynamic size -1, got %d", array.CairoSize())
	}
}

// Test Result type
func TestResult(t *testing.T) {
	// Test Ok variant with uint64
	okResult := NewResultOk[uint64, string](42)
	
	data, err := okResult.MarshalCairo()
	if err != nil {
		t.Fatalf("MarshalCairo failed: %v", err)
	}
	if len(data) != 2 { // discriminant + value
		t.Errorf("Expected 2 felts, got %d", len(data))
	}
	if UintFromFelt(data[0]) != 0 { // Ok discriminant
		t.Errorf("Expected Ok discriminant 0, got %d", UintFromFelt(data[0]))
	}
	if UintFromFelt(data[1]) != 42 {
		t.Errorf("Expected value 42, got %d", UintFromFelt(data[1]))
	}

	okResult2 := &Result[uint64, string]{}
	err = okResult2.UnmarshalCairo(data)
	if err != nil {
		t.Fatalf("UnmarshalCairo failed: %v", err)
	}
	if !okResult2.IsOk || okResult2.Ok != 42 {
		t.Errorf("Expected Ok(42), got IsOk=%v, Ok=%d", okResult2.IsOk, okResult2.Ok)
	}

	// Test Err variant
	errResult := NewResultErr[uint64, uint64](999)
	
	data, err = errResult.MarshalCairo()
	if err != nil {
		t.Fatalf("MarshalCairo failed: %v", err)
	}
	if len(data) != 2 { // discriminant + value
		t.Errorf("Expected 2 felts, got %d", len(data))
	}
	if UintFromFelt(data[0]) != 1 { // Err discriminant
		t.Errorf("Expected Err discriminant 1, got %d", UintFromFelt(data[0]))
	}
	if UintFromFelt(data[1]) != 999 {
		t.Errorf("Expected error 999, got %d", UintFromFelt(data[1]))
	}

	errResult2 := &Result[uint64, uint64]{}
	err = errResult2.UnmarshalCairo(data)
	if err != nil {
		t.Fatalf("UnmarshalCairo failed: %v", err)
	}
	if errResult2.IsOk || errResult2.Err != 999 {
		t.Errorf("Expected Err(999), got IsOk=%v, Err=%d", errResult2.IsOk, errResult2.Err)
	}

	if okResult.CairoSize() != -1 {
		t.Errorf("Expected dynamic size -1, got %d", okResult.CairoSize())
	}
}

// Test Option type
func TestOption(t *testing.T) {
	// Test Some variant
	someOption := NewOptionSome[uint64](123)
	
	data, err := someOption.MarshalCairo()
	if err != nil {
		t.Fatalf("MarshalCairo failed: %v", err)
	}
	if len(data) != 2 { // discriminant + value
		t.Errorf("Expected 2 felts, got %d", len(data))
	}
	if UintFromFelt(data[0]) != 0 { // Some discriminant
		t.Errorf("Expected Some discriminant 0, got %d", UintFromFelt(data[0]))
	}
	if UintFromFelt(data[1]) != 123 {
		t.Errorf("Expected value 123, got %d", UintFromFelt(data[1]))
	}

	someOption2 := &Option[uint64]{}
	err = someOption2.UnmarshalCairo(data)
	if err != nil {
		t.Fatalf("UnmarshalCairo failed: %v", err)
	}
	if !someOption2.IsSome || someOption2.Value != 123 {
		t.Errorf("Expected Some(123), got IsSome=%v, Value=%d", someOption2.IsSome, someOption2.Value)
	}

	// Test None variant
	noneOption := NewOptionNone[uint64]()
	
	data, err = noneOption.MarshalCairo()
	if err != nil {
		t.Fatalf("MarshalCairo failed: %v", err)
	}
	if len(data) != 1 { // discriminant only
		t.Errorf("Expected 1 felt, got %d", len(data))
	}
	if UintFromFelt(data[0]) != 1 { // None discriminant
		t.Errorf("Expected None discriminant 1, got %d", UintFromFelt(data[0]))
	}

	noneOption2 := &Option[uint64]{}
	err = noneOption2.UnmarshalCairo(data)
	if err != nil {
		t.Fatalf("UnmarshalCairo failed: %v", err)
	}
	if noneOption2.IsSome {
		t.Errorf("Expected None, got Some")
	}

	if someOption.CairoSize() != -1 {
		t.Errorf("Expected dynamic size -1, got %d", someOption.CairoSize())
	}
}

// Test complex nested types with basic types only
func TestNestedTypes(t *testing.T) {
	// Test Result<uint64, uint64> with basic types
	result := NewResultOk[uint64, uint64](456)
	
	data, err := result.MarshalCairo()
	if err != nil {
		t.Fatalf("MarshalCairo failed: %v", err)
	}
	
	// Should have: Result discriminant(0) + value(456) = 2 felts
	if len(data) != 2 {
		t.Errorf("Expected 2 felts, got %d", len(data))
	}
	
	result2 := &Result[uint64, uint64]{}
	err = result2.UnmarshalCairo(data)
	if err != nil {
		t.Fatalf("UnmarshalCairo failed: %v", err)
	}
	
	if !result2.IsOk {
		t.Errorf("Expected Ok result")
	}
	if result2.Ok != 456 {
		t.Errorf("Expected 456, got %d", result2.Ok)
	}
}

// Benchmark basic types
func BenchmarkCairoFeltMarshal(b *testing.B) {
	felt := NewCairoFelt(FeltFromUint(42))
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, err := felt.MarshalCairo()
		if err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkCairoFeltUnmarshal(b *testing.B) {
	data := []*felt.Felt{FeltFromUint(42)}
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		felt := &CairoFelt{}
		err := felt.UnmarshalCairo(data)
		if err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkCairoUint256Marshal(b *testing.B) {
	u256 := NewCairoUint256(big.NewInt(123), big.NewInt(456))
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, err := u256.MarshalCairo()
		if err != nil {
			b.Fatal(err)
		}
	}
}