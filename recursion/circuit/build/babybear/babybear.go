package babybear

import (
	"math/big"

	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/math/emulated"
)

var MODULUS = new(big.Int).SetUint64(2013265921)

type Params struct{}

func (fp Params) NbLimbs() uint            { return 1 }
func (fp Params) BitsPerLimb() uint        { return 32 }
func (fp Params) IsPrime() bool            { return true }
func (fp Params) Modulus() *big.Int        { return MODULUS }
func (fp Params) NumElmsPerBN254Elm() uint { return 8 }

type Variable struct {
	Value *emulated.Element[Params]
}

type ExtensionVariable struct {
	value [4]*Variable
}

type Chip struct {
	api   frontend.API
	field *emulated.Field[Params]
}

func NewChip(api frontend.API) *Chip {
	field, err := emulated.NewField[Params](api)
	if err != nil {
		panic(err)
	}
	return &Chip{
		api:   api,
		field: field,
	}
}

func NewVariable(value int) *Variable {
	variable := emulated.ValueOf[Params](value)
	return &Variable{
		Value: &variable,
	}
}

func NewExtensionVariable(value [4]int) *ExtensionVariable {
	a := NewVariable(value[0])
	b := NewVariable(value[1])
	c := NewVariable(value[2])
	d := NewVariable(value[3])
	return &ExtensionVariable{value: [4]*Variable{a, b, c, d}}
}

func (c *Chip) Add(a, b *Variable) *Variable {
	return &Variable{
		Value: c.field.Add(a.Value, b.Value),
	}
}

func (c *Chip) Sub(a, b *Variable) *Variable {
	return &Variable{
		Value: c.field.Sub(a.Value, b.Value),
	}
}

func (c *Chip) Mul(a, b *Variable) *Variable {
	return &Variable{
		Value: c.field.Mul(a.Value, b.Value),
	}
}

func (c *Chip) Neg(a *Variable) *Variable {
	return &Variable{
		Value: c.field.Neg(a.Value),
	}
}

func (c *Chip) Inv(a *Variable) *Variable {
	return &Variable{
		Value: c.field.Inverse(a.Value),
	}
}

func (c *Chip) AssertEq(a, b *Variable) {
	c.field.AssertIsEqual(a.Value, b.Value)
}

func (c *Chip) AssertNe(a, b *Variable) {
	diff := c.field.Sub(a.Value, b.Value)
	isZero := c.field.IsZero(diff)
	c.api.AssertIsEqual(isZero, frontend.Variable(0))
}

func (c *Chip) Select(cond frontend.Variable, a, b *Variable) *Variable {
	return &Variable{
		Value: c.field.Select(cond, a.Value, b.Value),
	}
}

func (c *Chip) AddExtension(a, b *ExtensionVariable) *ExtensionVariable {
	v1 := c.Add(a.value[0], b.value[0])
	v2 := c.Add(a.value[1], b.value[1])
	v3 := c.Add(a.value[2], b.value[2])
	v4 := c.Add(a.value[3], b.value[3])
	return &ExtensionVariable{value: [4]*Variable{v1, v2, v3, v4}}
}

func (c *Chip) SubExtension(a, b *ExtensionVariable) *ExtensionVariable {
	v1 := c.Sub(a.value[0], b.value[0])
	v2 := c.Sub(a.value[1], b.value[1])
	v3 := c.Sub(a.value[2], b.value[2])
	v4 := c.Sub(a.value[3], b.value[3])
	return &ExtensionVariable{value: [4]*Variable{v1, v2, v3, v4}}
}

func (c *Chip) MulExtension(a, b *ExtensionVariable) *ExtensionVariable {
	w := NewVariable(11)
	v := [4]*Variable{
		NewVariable(0),
		NewVariable(0),
		NewVariable(0),
		NewVariable(0),
	}

	for i := 0; i < 4; i++ {
		for j := 0; j < 4; j++ {
			if i+j >= 4 {
				v[i+j-4] = c.Add(v[i+j-4], c.Mul(c.Mul(v[i], v[j]), w))
			} else {
				v[i+j] = c.Add(v[i+j], c.Mul(v[i], v[j]))
			}
		}
	}

	return &ExtensionVariable{value: v}
}

func (c *Chip) NegExtension(a *ExtensionVariable) *ExtensionVariable {
	v1 := c.Neg(a.value[0])
	v2 := c.Neg(a.value[1])
	v3 := c.Neg(a.value[2])
	v4 := c.Neg(a.value[3])
	return &ExtensionVariable{value: [4]*Variable{v1, v2, v3, v4}}
}

func (c *Chip) InvExtension(a *ExtensionVariable) *ExtensionVariable {
	v := [4]*Variable{
		NewVariable(0),
		NewVariable(0),
		NewVariable(0),
		NewVariable(0),
	}
	return &ExtensionVariable{value: v}
}

func (c *Chip) AssertEqExtension(a, b *ExtensionVariable) {
	c.AssertEq(a.value[0], b.value[0])
	c.AssertEq(a.value[1], b.value[1])
	c.AssertEq(a.value[2], b.value[2])
	c.AssertEq(a.value[3], b.value[3])
}

func (c *Chip) AssertNeExtension(a, b *ExtensionVariable) {
	v1 := c.field.Sub(a.value[0].Value, b.value[0].Value)
	v2 := c.field.Sub(a.value[1].Value, b.value[1].Value)
	v3 := c.field.Sub(a.value[2].Value, b.value[2].Value)
	v4 := c.field.Sub(a.value[3].Value, b.value[3].Value)
	isZero1 := c.field.IsZero(v1)
	isZero2 := c.field.IsZero(v2)
	isZero3 := c.field.IsZero(v3)
	isZero4 := c.field.IsZero(v4)
	isZero1AndZero2 := c.api.And(isZero1, isZero2)
	isZero3AndZero4 := c.api.And(isZero3, isZero4)
	isZeroAll := c.api.And(isZero1AndZero2, isZero3AndZero4)
	c.api.AssertIsEqual(isZeroAll, frontend.Variable(0))
}

func (c *Chip) SelectExtension(cond frontend.Variable, a, b *ExtensionVariable) *ExtensionVariable {
	v1 := c.Select(cond, a.value[0], b.value[0])
	v2 := c.Select(cond, a.value[1], b.value[1])
	v3 := c.Select(cond, a.value[2], b.value[2])
	v4 := c.Select(cond, a.value[3], b.value[3])
	return &ExtensionVariable{value: [4]*Variable{v1, v2, v3, v4}}
}

func (c *Chip) SplitIntoBabyBear(in frontend.Variable) [3]Variable {
	bits := c.api.ToBinary(in)
	x := frontend.Variable(0)
	y := frontend.Variable(0)
	z := frontend.Variable(0)
	power := frontend.Variable(1)

	for i := 0; i < 32; i++ {
		x = c.api.Add(x, c.api.Mul(bits[i], power))
		y = c.api.Add(y, c.api.Mul(bits[i+32], power))
		z = c.api.Add(z, c.api.Mul(bits[i+64], power))
		power = c.api.Mul(power, frontend.Variable(2))
	}

	x1 := c.field.NewElement(x)
	y1 := c.field.NewElement(y)
	z1 := c.field.NewElement(z)

	return [3]Variable{{Value: x1}, {Value: y1}, {Value: z1}}
}

func (c *Chip) ToBinary(in *Variable) []frontend.Variable {
	return c.field.ToBits(in.Value)
}
