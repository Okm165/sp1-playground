// This file is auto-generated by sp1-recursion-compiler.
package main

import (
	"github.com/consensys/gnark/frontend"
	"github.com/succinctlabs/sp1-recursion-gnark/babybear"
)

type Circuit struct {
	X frontend.Variable
	Y frontend.Variable
}

func (circuit *Circuit) Define(api frontend.API) error {
	fieldChip := babybear.NewChip(api)
	
	// Variables.
	var var0 frontend.Variable
	var felt0 *babybear.Variable
	var felt1 *babybear.Variable
	var backend0 frontend.Variable
	var felt2 *babybear.Variable
	var backend1 frontend.Variable
	
	// Operations.
	var0 = frontend.Variable(0)
	felt0 = babybear.NewVariable(0)
	felt1 = babybear.NewVariable(1)
	for i := 0; i < 12; i++ {
		felt2 = fieldChip.Add(felt1, babybear.NewVariable(0))
		felt1 = fieldChip.Add(felt0, felt1)
		felt0 = fieldChip.Add(felt2, babybear.NewVariable(0))
	}
	fieldChip.AssertEq(felt0, babybear.NewVariable(144))
	backend0 = api.IsZero(api.Sub(var0, var0))
	felt0 = fieldChip.Select(backend0,  fieldChip.Add(felt1, babybear.NewVariable(0)), felt0)
	felt0 = fieldChip.Select(backend0,  fieldChip.Add(felt0, felt1), felt0)
	backend1 = api.Sub(frontend.Variable(1), api.IsZero(api.Sub(var0, var0)))
	felt0 = fieldChip.Select(backend1,  fieldChip.Add(felt1, babybear.NewVariable(0)), felt0)
	felt0 = fieldChip.Select(backend1,  fieldChip.Add(felt0, felt1), felt0)
	return nil
}
