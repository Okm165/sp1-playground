package poseidon2

import (
	"github.com/consensys/gnark/frontend"
	"github.com/succinctlabs/sp1-recursion-gnark/sp1/babybear"
)

const BABYBEAR_WIDTH = 16
const BABYBEAR_NUM_EXTERNAL_ROUNDS = 8
const BABYBEAR_NUM_INTERNAL_ROUNDS = 13
const BABYBEAR_DEGREE = 7

type Poseidon2BabyBearChip struct {
	api                 frontend.API
	fieldApi            *babybear.Chip
	internalLinearLayer [BABYBEAR_WIDTH]babybear.Variable
}

func NewPoseidon2BabyBearChip(api frontend.API) *Poseidon2BabyBearChip {
	return &Poseidon2BabyBearChip{
		api:      api,
		fieldApi: babybear.NewChip(api),
		internalLinearLayer: [BABYBEAR_WIDTH]babybear.Variable{
			babybear.NewF("2013265919"),
			babybear.NewF("1"),
			babybear.NewF("2"),
			babybear.NewF("4"),
			babybear.NewF("8"),
			babybear.NewF("16"),
			babybear.NewF("32"),
			babybear.NewF("64"),
			babybear.NewF("128"),
			babybear.NewF("256"),
			babybear.NewF("512"),
			babybear.NewF("1024"),
			babybear.NewF("2048"),
			babybear.NewF("4096"),
			babybear.NewF("8192"),
			babybear.NewF("32768"),
		},
	}
}

func (p *Poseidon2BabyBearChip) PermuteMut(state *[BABYBEAR_WIDTH]babybear.Variable) {
	// The initial linear layer.
	p.externalLinearLayer(state)

	// The first half of the external rounds.
	// rounds := BABYBEAR_NUM_EXTERNAL_ROUNDS + BABYBEAR_NUM_INTERNAL_ROUNDS
	roundsFBeggining := BABYBEAR_NUM_EXTERNAL_ROUNDS / 2
	for r := 0; r < roundsFBeggining; r++ {
		p.addRc(state, RC16[r])
		p.sbox(state)
		p.externalLinearLayer(state)
		if r == 0 {
			break
		}
	}

	// // The internal rounds.
	// p_end := roundsFBeggining + BABYBEAR_NUM_INTERNAL_ROUNDS
	// for r := roundsFBeggining; r < p_end; r++ {
	// 	state[0] = p.fieldApi.AddF(state[0], RC16[r][0])
	// 	state[0] = p.sboxP(state[0])
	// 	p.diffusionPermuteMut(state)
	// }

	// // The second half of the external rounds.
	// for r := p_end; r < rounds; r++ {
	// 	p.addRc(state, RC16[r])
	// 	p.sbox(state)
	// 	p.matrixPermuteMut(state)
	// }
}

func (p *Poseidon2BabyBearChip) addRc(state *[BABYBEAR_WIDTH]babybear.Variable, rc [BABYBEAR_WIDTH]babybear.Variable) {
	for i := 0; i < BABYBEAR_WIDTH; i++ {
		state[i] = p.fieldApi.AddF(state[i], rc[i])
	}
}

func (p *Poseidon2BabyBearChip) sboxP(input babybear.Variable) babybear.Variable {
	if BABYBEAR_DEGREE != 7 {
		panic("DEGREE is assumed to be 7")
	}

	squared := p.fieldApi.MulF(input, input)
	input4 := p.fieldApi.MulF(squared, squared)
	input6 := p.fieldApi.MulF(squared, input4)
	return p.fieldApi.MulF(input6, input)
}

func (p *Poseidon2BabyBearChip) sbox(state *[BABYBEAR_WIDTH]babybear.Variable) {
	for i := 0; i < BABYBEAR_WIDTH; i++ {
		state[i] = p.sboxP(state[i])
	}
}

func (p *Poseidon2BabyBearChip) mdsLightPermutation4x4(state []babybear.Variable) {
	t01 := p.fieldApi.AddF(state[0], state[1])
	t23 := p.fieldApi.AddF(state[2], state[3])
	t0123 := p.fieldApi.AddF(t01, t23)
	t01123 := p.fieldApi.AddF(t0123, state[1])
	t01233 := p.fieldApi.AddF(t0123, state[3])
	state[3] = p.fieldApi.AddF(t01233, p.fieldApi.MulFConst(state[0], 2))
	state[1] = p.fieldApi.AddF(t01123, p.fieldApi.MulFConst(state[2], 2))
	state[0] = p.fieldApi.AddF(t01123, t01)
	state[2] = p.fieldApi.AddF(t01233, t23)
}

func (p *Poseidon2BabyBearChip) externalLinearLayer(state *[BABYBEAR_WIDTH]babybear.Variable) {
	for i := 0; i < BABYBEAR_WIDTH; i += 4 {
		p.mdsLightPermutation4x4(state[i : i+4])
	}

	sums := [4]babybear.Variable{
		state[0],
		state[1],
		state[2],
		state[3],
	}
	for i := 4; i < BABYBEAR_WIDTH; i += 4 {
		sums[0] = p.fieldApi.AddF(sums[0], state[i])
		sums[1] = p.fieldApi.AddF(sums[1], state[i+1])
		sums[2] = p.fieldApi.AddF(sums[2], state[i+2])
		sums[3] = p.fieldApi.AddF(sums[3], state[i+3])
	}

	for i := 0; i < BABYBEAR_WIDTH; i++ {
		state[i] = p.fieldApi.AddF(state[i], sums[i%4])
	}
}

func (p *Poseidon2BabyBearChip) diffusionPermuteMut(state *[BABYBEAR_WIDTH]babybear.Variable) {
	sum := babybear.NewF("0")
	for i := 0; i < BABYBEAR_WIDTH; i++ {
		sum = p.fieldApi.AddF(sum, state[i])
	}

	for i := 0; i < BABYBEAR_WIDTH; i++ {
		state[i] = p.fieldApi.MulF(state[i], p.internalLinearLayer[i])
		state[i] = p.fieldApi.AddF(state[i], sum)
	}
}
