#!/usr/bin/env bats
# shellcheck disable=SC2016

@test "Trivial 2+2=4" {
	run ./rc '2+2'
	[ "${lines[0]}" = "4" ]
	[ "$status" -eq 0 ]
}

@test "Float vs rational" {
	run ./rc '1/0.5 = 1/(1/2)'
	[ "${lines[0]}" = "2" ]
	[ "$status" -eq 0 ]
}

@test "Square root" {
	run ./rc 'sqrt(4) = 4/2'
	[ "${lines[0]}" = "2" ]
	[ "$status" -eq 0 ]
}

@test "Variables" {
	run ./rc 'x=1;x*(1+1)=y;y/x'
	[ "${lines[0]}" = "2" ]
	[ "$status" -eq 0 ]
}

@test "Compare 1<2<3" {
	run ./rc '1<2<3'
	[ "${lines[0]}" = "3" ]
	[ "$status" -eq 0 ]
}

@test "Quiet flag and print result of last line" {
	run ./rc --quiet '2+2; print({_})'
	[ "${lines[0]}" = "4" ]
	[ "$status" -eq 0 ]
}

@test "Print formatting uses correct spaces" {
	run ./rc --quiet 'print(  {4/2}+2 = \{\| {2+2} \|\} )'
	[ "${lines[0]}" = "  2+2 = {| 4 |} " ]
	[ "$status" -eq 0 ]
}

@test "Custom errors" {
	run ./rc --quiet 'error(2+2 != {9/2})'
	[ "${lines[0]}" = "error: 2+2 != 9/2" ]
	[ "$status" -eq 2 ]
}

@test "Binomial coefficient" {
	run ./rc -f examples/binomial.rc
	[ "$status" -eq 0 ]
}

@test "Factorial" {
	run ./rc -f examples/factorial.rc
	[ "$status" -eq 0 ]
}

@test "Fibonacci sequence" {
	run ./rc -f examples/fibonacci.rc
	[ "$status" -eq 0 ]
}

@test "Gaussian" {
	run ./rc -f examples/gaussian.rc
	[ "$status" -eq 0 ]
}

@test "Pipe operator replacement" {
	run ./rc -f examples/pipe.rc
	[ "${lines[0]}" = "4" ]
	[ "$status" -eq 0 ]
}

@test "Implementing isqrt using custom map function" {
	run ./rc -f examples/isqrt_map.rc
	[ "$status" -eq 0 ]
}

@test "Interval tests" {
	run ./rc -f examples/interval_tests.rc
	[ "$status" -eq 0 ]
}
