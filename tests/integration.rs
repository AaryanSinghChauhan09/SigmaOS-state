use sigmaos_state::*;

#[test]
fn component_is_constructible_and_runnable() {
    // Arrange
    let c = sigmaos_state::new("sigmaos-state").expect("valid name");
    // Act
    let out = c.run().expect("run succeeds");
    // Assert
    assert!(out.contains("sigmaos-state"));
}
