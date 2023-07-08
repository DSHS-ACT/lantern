use lantern::run;

fn main() {
    // run 함수 종료까지 기다리기
    pollster::block_on(run());
}
