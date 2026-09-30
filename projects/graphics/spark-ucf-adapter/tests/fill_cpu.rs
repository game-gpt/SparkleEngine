//! UCF CPU Fill 切片：图结构可测，不依赖窗口泵。

use spark_ucf_adapter::{fill_rgba8_cpu_fallback, SparkComputePass, SparkUcfExecutor};

#[test]
fn color_fill_via_ucf_matches_fallback() {
    let pass = SparkComputePass::color_fill("clear", 4, 2, [10, 20, 30, 255]);
    let expected = fill_rgba8_cpu_fallback(pass.width, pass.height, pass.rgba);

    let mut exec = SparkUcfExecutor::open_cpu();
    assert!(exec.capabilities().contains_backend("cpu"));
    let got = exec.run_color_fill(&pass).expect("ucf fill");
    assert_eq!(got, expected);
    assert!(exec.diagnostics().contains_kind("task_submit"));
}
