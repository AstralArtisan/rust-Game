use bevy::prelude::Resource;

/// 临时测试设施：开启后玩家死亡不进 GameOver，而是原地满血复活 + 短暂无敌。
/// 仅供组员测试使用，测试结束后整套机制将被删除（见 docs/test_mode_temp.md）。
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct TestMode(pub bool);
