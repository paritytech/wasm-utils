(module
	(func (param $x i32) (result i32)
		(if (result i32)
			(i32.const 1)
			(then (i32.add (get_local $x) (i32.const 1)))
			(else (i32.popcnt (get_local $x)))
		)
	)
)
