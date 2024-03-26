package verifier

import (
	"testing"

	"github.com/consensys/gnark/test"
)

func TestCircuit(t *testing.T) {
	assert := test.NewAssert(t)
	var circuit Circuit
	assert.ProverSucceeded(&circuit, &Circuit{
		X: 0,
		Y: 0,
	})
}
