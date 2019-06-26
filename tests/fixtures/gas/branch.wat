(module
	(func $fibonacci_with_break (result i32)
		(local $x i32) (local $y i32)

		(block $unrolled_loop
			(set_local $x (i32.const 0))
			(set_local $y (i32.const 1))

			get_local $x
			get_local $y
			tee_local $x
			i32.add
			set_local $y

			i32.const 1
			br_if $unrolled_loop

			get_local $x
			get_local $y
			tee_local $x
			i32.add
			set_local $y
		)

		get_local $y
	)
)
