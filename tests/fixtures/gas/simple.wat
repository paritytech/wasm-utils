(module
	(func (export "simple")
		(if (i32.const 1)
			(loop
				i32.const 123
				drop
			)
		)
	)

	(func
		block
		end
	)
)
