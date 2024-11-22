test:
  cargo test
benchmark *args='':
	python3 tools/benchmark.py .benchmark {{args}}
callgrind *args='':
	rm -f callgrind.out*
	cargo build --profile profiling
	valgrind --tool=callgrind --dump-instr=yes ./target/profiling/guitar_tab {{args}}
	kcachegrind callgrind.out* 
profile *args='':
  cargo build --profile profiling
  sudo samply record --rate 5000 ./target/profiling/guitar_tab {{args}} 
