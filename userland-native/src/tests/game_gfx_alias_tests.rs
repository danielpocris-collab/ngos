use super::*;

#[test]
fn native_shell_translates_api_specific_graphics_aliases_for_major_families() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/dx\nmkdir-path /games/gl\nmkdir-path /games/metal\nmkdir-path /games/web\nmkfile-path /games/dx.manifest\nappend-line /games/dx.manifest title=DX Arena\nappend-line /games/dx.manifest slug=dx-arena\nappend-line /games/dx.manifest exec=/bin/worker\nappend-line /games/dx.manifest cwd=/games/dx\nappend-line /games/dx.manifest gfx.api=directx12\nappend-line /games/dx.manifest gfx.backend=vulkan\nappend-line /games/dx.manifest gfx.profile=frame-pace\nappend-line /games/dx.manifest audio.backend=native-mixer\nappend-line /games/dx.manifest audio.profile=stereo\nappend-line /games/dx.manifest input.backend=native-input\nappend-line /games/dx.manifest input.profile=kbm\nappend-line /games/dx.manifest shim.prefix=/compat/dx\nappend-line /games/dx.manifest shim.saves=/saves/dx\nappend-line /games/dx.manifest shim.cache=/cache/dx\nmkfile-path /games/gl.manifest\nappend-line /games/gl.manifest title=GL Arena\nappend-line /games/gl.manifest slug=gl-arena\nappend-line /games/gl.manifest exec=/bin/worker\nappend-line /games/gl.manifest cwd=/games/gl\nappend-line /games/gl.manifest gfx.api=opengl\nappend-line /games/gl.manifest gfx.backend=vulkan\nappend-line /games/gl.manifest gfx.profile=compat\nappend-line /games/gl.manifest audio.backend=native-mixer\nappend-line /games/gl.manifest audio.profile=stereo\nappend-line /games/gl.manifest input.backend=native-input\nappend-line /games/gl.manifest input.profile=kbm\nappend-line /games/gl.manifest shim.prefix=/compat/gl\nappend-line /games/gl.manifest shim.saves=/saves/gl\nappend-line /games/gl.manifest shim.cache=/cache/gl\nmkfile-path /games/metal.manifest\nappend-line /games/metal.manifest title=Metal Arena\nappend-line /games/metal.manifest slug=metal-arena\nappend-line /games/metal.manifest exec=/bin/worker\nappend-line /games/metal.manifest cwd=/games/metal\nappend-line /games/metal.manifest gfx.api=metal\nappend-line /games/metal.manifest gfx.backend=vulkan\nappend-line /games/metal.manifest gfx.profile=compat\nappend-line /games/metal.manifest audio.backend=native-mixer\nappend-line /games/metal.manifest audio.profile=stereo\nappend-line /games/metal.manifest input.backend=native-input\nappend-line /games/metal.manifest input.profile=kbm\nappend-line /games/metal.manifest shim.prefix=/compat/metal\nappend-line /games/metal.manifest shim.saves=/saves/metal\nappend-line /games/metal.manifest shim.cache=/cache/metal\nmkfile-path /games/web.manifest\nappend-line /games/web.manifest title=Web Arena\nappend-line /games/web.manifest slug=web-arena\nappend-line /games/web.manifest exec=/bin/worker\nappend-line /games/web.manifest cwd=/games/web\nappend-line /games/web.manifest gfx.api=webgpu\nappend-line /games/web.manifest gfx.backend=vulkan\nappend-line /games/web.manifest gfx.profile=compat\nappend-line /games/web.manifest audio.backend=native-mixer\nappend-line /games/web.manifest audio.profile=stereo\nappend-line /games/web.manifest input.backend=native-input\nappend-line /games/web.manifest input.profile=kbm\nappend-line /games/web.manifest shim.prefix=/compat/web\nappend-line /games/web.manifest shim.saves=/saves/web\nappend-line /games/web.manifest shim.cache=/cache/web\ngame-launch /games/dx.manifest\ngame-launch /games/gl.manifest\ngame-launch /games/metal.manifest\ngame-launch /games/web.manifest\nmkfile-path /games/dx.frame\nappend-line /games/dx.frame surface=1280x720\nappend-line /games/dx.frame frame=dx-api-001\nappend-line /games/dx.frame queue=graphics\nappend-line /games/dx.frame present-mode=mailbox\nappend-line /games/dx.frame completion=fire-and-forget\nappend-line /games/dx.frame dx-clear-rtv=000000ff\nappend-line /games/dx.frame dx-fill-rect=0,0,1280,720,112233ff\nappend-line /games/dx.frame dx-copy-resource=hud,0,0,1280,64\nappend-line /games/dx.frame dx-present=0,0,1280,720\nmkfile-path /games/gl.frame\nappend-line /games/gl.frame surface=800x600\nappend-line /games/gl.frame frame=gl-api-001\nappend-line /games/gl.frame queue=graphics\nappend-line /games/gl.frame present-mode=fifo\nappend-line /games/gl.frame completion=fire-and-forget\nappend-line /games/gl.frame gl-clear=101010ff\nappend-line /games/gl.frame gl-draw-line=0,0,799,599,ffffffff\nappend-line /games/gl.frame gl-swap-buffers=0,0,800,600\nmkfile-path /games/metal.frame\nappend-line /games/metal.frame surface=1024x768\nappend-line /games/metal.frame frame=metal-api-001\nappend-line /games/metal.frame queue=graphics\nappend-line /games/metal.frame present-mode=fifo\nappend-line /games/metal.frame completion=fire-and-forget\nappend-line /games/metal.frame metal-clear=001122ff\nappend-line /games/metal.frame metal-fill-rounded-rect=10,10,200,120,12,88aaffff\nappend-line /games/metal.frame metal-present-drawable=0,0,1024,768\nmkfile-path /games/web.frame\nappend-line /games/web.frame surface=640x360\nappend-line /games/web.frame frame=web-api-001\nappend-line /games/web.frame queue=graphics\nappend-line /games/web.frame present-mode=immediate\nappend-line /games/web.frame completion=fire-and-forget\nappend-line /games/web.frame webgpu-clear-pass=0f0f0fff\nappend-line /games/web.frame webgpu-copy-texture=backbuffer,0,0,640,360\nappend-line /games/web.frame webgpu-present=0,0,640,360\ngame-gfx-translate 77 /games/dx.frame\ngame-gfx-translate 78 /games/gl.frame\ngame-gfx-translate 79 /games/metal.frame\ngame-gfx-translate 80 /games/web.frame\ngame-gfx-next 77\ngame-gfx-next 78\ngame-gfx-next 79\ngame-gfx-next 80\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("game.gfx.translate pid=77 frame=dx-api-001 ops=4"));
    assert!(stdout.contains("game.gfx.translate pid=78 frame=gl-api-001 ops=3"));
    assert!(stdout.contains("game.gfx.translate pid=79 frame=metal-api-001 ops=3"));
    assert!(stdout.contains("game.gfx.translate pid=80 frame=web-api-001 ops=3"));
    assert!(
        stdout.contains("pid=77 frame=dx-api-001 ops=4 bytes=")
            && stdout.contains("api=directx12 translation=compat-to-vulkan")
    );
    assert!(
        stdout.contains("pid=78 frame=gl-api-001 ops=3 bytes=")
            && stdout.contains("api=opengl translation=compat-to-vulkan")
    );
    assert!(
        stdout.contains("pid=79 frame=metal-api-001 ops=3 bytes=")
            && stdout.contains("api=metal translation=compat-to-vulkan")
    );
    assert!(
        stdout.contains("pid=80 frame=web-api-001 ops=3 bytes=")
            && stdout.contains("api=webgpu translation=compat-to-vulkan")
    );
    assert!(stdout.contains(
        "game.gfx.next pid=77 frame=dx-api-001 api=directx12 translation=compat-to-vulkan"
    ));
    assert!(
        stdout.contains(
            "game.gfx.next pid=78 frame=gl-api-001 api=opengl translation=compat-to-vulkan"
        )
    );
    assert!(stdout.contains(
        "game.gfx.next pid=79 frame=metal-api-001 api=metal translation=compat-to-vulkan"
    ));
    assert!(stdout.contains(
        "game.gfx.next pid=80 frame=web-api-001 api=webgpu translation=compat-to-vulkan"
    ));
}

#[test]
fn native_shell_refuses_graphics_alias_from_wrong_api_family() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/gl\nmkfile-path /games/gl.manifest\nappend-line /games/gl.manifest title=GL Arena\nappend-line /games/gl.manifest slug=gl-arena\nappend-line /games/gl.manifest exec=/bin/worker\nappend-line /games/gl.manifest cwd=/games/gl\nappend-line /games/gl.manifest gfx.api=opengl\nappend-line /games/gl.manifest gfx.backend=vulkan\nappend-line /games/gl.manifest gfx.profile=compat\nappend-line /games/gl.manifest audio.backend=native-mixer\nappend-line /games/gl.manifest audio.profile=stereo\nappend-line /games/gl.manifest input.backend=native-input\nappend-line /games/gl.manifest input.profile=kbm\nappend-line /games/gl.manifest shim.prefix=/compat/gl\nappend-line /games/gl.manifest shim.saves=/saves/gl\nappend-line /games/gl.manifest shim.cache=/cache/gl\ngame-launch /games/gl.manifest\nmkfile-path /games/gl.bad\nappend-line /games/gl.bad surface=800x600\nappend-line /games/gl.bad frame=gl-bad-001\nappend-line /games/gl.bad queue=graphics\nappend-line /games/gl.bad present-mode=fifo\nappend-line /games/gl.bad completion=fire-and-forget\nappend-line /games/gl.bad dx-clear-rtv=ff0000ff\ngame-gfx-translate 77 /games/gl.bad\nlast-status\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("last-status=291"));
    assert!(!stdout.contains("game.gfx.translate pid=77"));
}

#[test]
fn native_shell_translates_api_specific_graphics_aliases_for_all_supported_families() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/d3d9\nmkdir-path /games/d3d10\nmkdir-path /games/dx11\nmkdir-path /games/dx12\nmkdir-path /games/gl\nmkdir-path /games/gles\nmkdir-path /games/metal\nmkdir-path /games/vk\nmkdir-path /games/web\nmkdir-path /games/wgpu\nmkfile-path /games/d3d9.manifest\nappend-line /games/d3d9.manifest title=D3D9 Arena\nappend-line /games/d3d9.manifest slug=d3d9-arena\nappend-line /games/d3d9.manifest exec=/bin/worker\nappend-line /games/d3d9.manifest cwd=/games/d3d9\nappend-line /games/d3d9.manifest gfx.api=direct3d9\nappend-line /games/d3d9.manifest gfx.backend=vulkan\nappend-line /games/d3d9.manifest gfx.profile=compat\nappend-line /games/d3d9.manifest audio.backend=native-mixer\nappend-line /games/d3d9.manifest audio.profile=stereo\nappend-line /games/d3d9.manifest input.backend=native-input\nappend-line /games/d3d9.manifest input.profile=kbm\nappend-line /games/d3d9.manifest shim.prefix=/compat/d3d9\nappend-line /games/d3d9.manifest shim.saves=/saves/d3d9\nappend-line /games/d3d9.manifest shim.cache=/cache/d3d9\nmkfile-path /games/d3d10.manifest\nappend-line /games/d3d10.manifest title=D3D10 Arena\nappend-line /games/d3d10.manifest slug=d3d10-arena\nappend-line /games/d3d10.manifest exec=/bin/worker\nappend-line /games/d3d10.manifest cwd=/games/d3d10\nappend-line /games/d3d10.manifest gfx.api=direct3d10\nappend-line /games/d3d10.manifest gfx.backend=vulkan\nappend-line /games/d3d10.manifest gfx.profile=compat\nappend-line /games/d3d10.manifest audio.backend=native-mixer\nappend-line /games/d3d10.manifest audio.profile=stereo\nappend-line /games/d3d10.manifest input.backend=native-input\nappend-line /games/d3d10.manifest input.profile=kbm\nappend-line /games/d3d10.manifest shim.prefix=/compat/d3d10\nappend-line /games/d3d10.manifest shim.saves=/saves/d3d10\nappend-line /games/d3d10.manifest shim.cache=/cache/d3d10\nmkfile-path /games/dx11.manifest\nappend-line /games/dx11.manifest title=DX11 Arena\nappend-line /games/dx11.manifest slug=dx11-arena\nappend-line /games/dx11.manifest exec=/bin/worker\nappend-line /games/dx11.manifest cwd=/games/dx11\nappend-line /games/dx11.manifest gfx.api=directx11\nappend-line /games/dx11.manifest gfx.backend=vulkan\nappend-line /games/dx11.manifest gfx.profile=compat\nappend-line /games/dx11.manifest audio.backend=native-mixer\nappend-line /games/dx11.manifest audio.profile=stereo\nappend-line /games/dx11.manifest input.backend=native-input\nappend-line /games/dx11.manifest input.profile=kbm\nappend-line /games/dx11.manifest shim.prefix=/compat/dx11\nappend-line /games/dx11.manifest shim.saves=/saves/dx11\nappend-line /games/dx11.manifest shim.cache=/cache/dx11\nmkfile-path /games/dx12.manifest\nappend-line /games/dx12.manifest title=DX12 Arena\nappend-line /games/dx12.manifest slug=dx12-arena\nappend-line /games/dx12.manifest exec=/bin/worker\nappend-line /games/dx12.manifest cwd=/games/dx12\nappend-line /games/dx12.manifest gfx.api=directx12\nappend-line /games/dx12.manifest gfx.backend=vulkan\nappend-line /games/dx12.manifest gfx.profile=compat\nappend-line /games/dx12.manifest audio.backend=native-mixer\nappend-line /games/dx12.manifest audio.profile=stereo\nappend-line /games/dx12.manifest input.backend=native-input\nappend-line /games/dx12.manifest input.profile=kbm\nappend-line /games/dx12.manifest shim.prefix=/compat/dx12\nappend-line /games/dx12.manifest shim.saves=/saves/dx12\nappend-line /games/dx12.manifest shim.cache=/cache/dx12\nmkfile-path /games/gl.manifest\nappend-line /games/gl.manifest title=GL Arena\nappend-line /games/gl.manifest slug=gl-arena\nappend-line /games/gl.manifest exec=/bin/worker\nappend-line /games/gl.manifest cwd=/games/gl\nappend-line /games/gl.manifest gfx.api=opengl\nappend-line /games/gl.manifest gfx.backend=vulkan\nappend-line /games/gl.manifest gfx.profile=compat\nappend-line /games/gl.manifest audio.backend=native-mixer\nappend-line /games/gl.manifest audio.profile=stereo\nappend-line /games/gl.manifest input.backend=native-input\nappend-line /games/gl.manifest input.profile=kbm\nappend-line /games/gl.manifest shim.prefix=/compat/gl\nappend-line /games/gl.manifest shim.saves=/saves/gl\nappend-line /games/gl.manifest shim.cache=/cache/gl\nmkfile-path /games/gles.manifest\nappend-line /games/gles.manifest title=GLES Arena\nappend-line /games/gles.manifest slug=gles-arena\nappend-line /games/gles.manifest exec=/bin/worker\nappend-line /games/gles.manifest cwd=/games/gles\nappend-line /games/gles.manifest gfx.api=opengles\nappend-line /games/gles.manifest gfx.backend=vulkan\nappend-line /games/gles.manifest gfx.profile=compat\nappend-line /games/gles.manifest audio.backend=native-mixer\nappend-line /games/gles.manifest audio.profile=stereo\nappend-line /games/gles.manifest input.backend=native-input\nappend-line /games/gles.manifest input.profile=kbm\nappend-line /games/gles.manifest shim.prefix=/compat/gles\nappend-line /games/gles.manifest shim.saves=/saves/gles\nappend-line /games/gles.manifest shim.cache=/cache/gles\nmkfile-path /games/metal.manifest\nappend-line /games/metal.manifest title=Metal Arena\nappend-line /games/metal.manifest slug=metal-arena\nappend-line /games/metal.manifest exec=/bin/worker\nappend-line /games/metal.manifest cwd=/games/metal\nappend-line /games/metal.manifest gfx.api=metal\nappend-line /games/metal.manifest gfx.backend=vulkan\nappend-line /games/metal.manifest gfx.profile=compat\nappend-line /games/metal.manifest audio.backend=native-mixer\nappend-line /games/metal.manifest audio.profile=stereo\nappend-line /games/metal.manifest input.backend=native-input\nappend-line /games/metal.manifest input.profile=kbm\nappend-line /games/metal.manifest shim.prefix=/compat/metal\nappend-line /games/metal.manifest shim.saves=/saves/metal\nappend-line /games/metal.manifest shim.cache=/cache/metal\nmkfile-path /games/vk.manifest\nappend-line /games/vk.manifest title=VK Arena\nappend-line /games/vk.manifest slug=vk-arena\nappend-line /games/vk.manifest exec=/bin/worker\nappend-line /games/vk.manifest cwd=/games/vk\nappend-line /games/vk.manifest gfx.api=vulkan\nappend-line /games/vk.manifest gfx.backend=vulkan\nappend-line /games/vk.manifest gfx.profile=native\nappend-line /games/vk.manifest audio.backend=native-mixer\nappend-line /games/vk.manifest audio.profile=stereo\nappend-line /games/vk.manifest input.backend=native-input\nappend-line /games/vk.manifest input.profile=kbm\nappend-line /games/vk.manifest shim.prefix=/compat/vk\nappend-line /games/vk.manifest shim.saves=/saves/vk\nappend-line /games/vk.manifest shim.cache=/cache/vk\nmkfile-path /games/web.manifest\nappend-line /games/web.manifest title=Web Arena\nappend-line /games/web.manifest slug=web-arena\nappend-line /games/web.manifest exec=/bin/worker\nappend-line /games/web.manifest cwd=/games/web\nappend-line /games/web.manifest gfx.api=webgpu\nappend-line /games/web.manifest gfx.backend=vulkan\nappend-line /games/web.manifest gfx.profile=compat\nappend-line /games/web.manifest audio.backend=native-mixer\nappend-line /games/web.manifest audio.profile=stereo\nappend-line /games/web.manifest input.backend=native-input\nappend-line /games/web.manifest input.profile=kbm\nappend-line /games/web.manifest shim.prefix=/compat/web\nappend-line /games/web.manifest shim.saves=/saves/web\nappend-line /games/web.manifest shim.cache=/cache/web\nmkfile-path /games/wgpu.manifest\nappend-line /games/wgpu.manifest title=WGPU Arena\nappend-line /games/wgpu.manifest slug=wgpu-arena\nappend-line /games/wgpu.manifest exec=/bin/worker\nappend-line /games/wgpu.manifest cwd=/games/wgpu\nappend-line /games/wgpu.manifest gfx.api=wgpu\nappend-line /games/wgpu.manifest gfx.backend=vulkan\nappend-line /games/wgpu.manifest gfx.profile=compat\nappend-line /games/wgpu.manifest audio.backend=native-mixer\nappend-line /games/wgpu.manifest audio.profile=stereo\nappend-line /games/wgpu.manifest input.backend=native-input\nappend-line /games/wgpu.manifest input.profile=kbm\nappend-line /games/wgpu.manifest shim.prefix=/compat/wgpu\nappend-line /games/wgpu.manifest shim.saves=/saves/wgpu\nappend-line /games/wgpu.manifest shim.cache=/cache/wgpu\ngame-launch /games/d3d9.manifest\ngame-launch /games/d3d10.manifest\ngame-launch /games/dx11.manifest\ngame-launch /games/dx12.manifest\ngame-launch /games/gl.manifest\ngame-launch /games/gles.manifest\ngame-launch /games/metal.manifest\ngame-launch /games/vk.manifest\ngame-launch /games/web.manifest\ngame-launch /games/wgpu.manifest\nmkfile-path /games/d3d9.frame\nappend-line /games/d3d9.frame surface=640x480\nappend-line /games/d3d9.frame frame=d3d9-api-001\nappend-line /games/d3d9.frame queue=graphics\nappend-line /games/d3d9.frame present-mode=fifo\nappend-line /games/d3d9.frame completion=fire-and-forget\nappend-line /games/d3d9.frame dx-clear-rtv=111111ff\nappend-line /games/d3d9.frame dx-draw-sprite=hero,32,32,64,64\nappend-line /games/d3d9.frame dx-present=0,0,640,480\nmkfile-path /games/d3d10.frame\nappend-line /games/d3d10.frame surface=800x600\nappend-line /games/d3d10.frame frame=d3d10-api-001\nappend-line /games/d3d10.frame queue=graphics\nappend-line /games/d3d10.frame present-mode=fifo\nappend-line /games/d3d10.frame completion=fire-and-forget\nappend-line /games/d3d10.frame dx-clear-rtv=222222ff\nappend-line /games/d3d10.frame dx-draw-triangle=0,0,100,0,50,80,00ff00ff\nappend-line /games/d3d10.frame dx-present=0,0,800,600\nmkfile-path /games/dx11.frame\nappend-line /games/dx11.frame surface=960x540\nappend-line /games/dx11.frame frame=dx11-api-001\nappend-line /games/dx11.frame queue=graphics\nappend-line /games/dx11.frame present-mode=mailbox\nappend-line /games/dx11.frame completion=fire-and-forget\nappend-line /games/dx11.frame dx-clear-rtv=000000ff\nappend-line /games/dx11.frame dx-fill-rect=0,0,960,540,112233ff\nappend-line /games/dx11.frame dx-present=0,0,960,540\nmkfile-path /games/dx12.frame\nappend-line /games/dx12.frame surface=1280x720\nappend-line /games/dx12.frame frame=dx12-api-001\nappend-line /games/dx12.frame queue=graphics\nappend-line /games/dx12.frame present-mode=mailbox\nappend-line /games/dx12.frame completion=fire-and-forget\nappend-line /games/dx12.frame dx-clear-rtv=000000ff\nappend-line /games/dx12.frame dx-fill-rect=0,0,1280,720,112233ff\nappend-line /games/dx12.frame dx-copy-resource=hud,0,0,1280,64\nappend-line /games/dx12.frame dx-present=0,0,1280,720\nmkfile-path /games/gl.frame\nappend-line /games/gl.frame surface=800x600\nappend-line /games/gl.frame frame=gl-api-001\nappend-line /games/gl.frame queue=graphics\nappend-line /games/gl.frame present-mode=fifo\nappend-line /games/gl.frame completion=fire-and-forget\nappend-line /games/gl.frame gl-clear=101010ff\nappend-line /games/gl.frame gl-draw-line=0,0,799,599,ffffffff\nappend-line /games/gl.frame gl-swap-buffers=0,0,800,600\nmkfile-path /games/gles.frame\nappend-line /games/gles.frame surface=480x320\nappend-line /games/gles.frame frame=gles-api-001\nappend-line /games/gles.frame queue=graphics\nappend-line /games/gles.frame present-mode=immediate\nappend-line /games/gles.frame completion=fire-and-forget\nappend-line /games/gles.frame gles-clear=333333ff\nappend-line /games/gles.frame gles-draw-ellipse=10,10,100,60,ff00ffff\nappend-line /games/gles.frame gles-swap-buffers=0,0,480,320\nmkfile-path /games/metal.frame\nappend-line /games/metal.frame surface=1024x768\nappend-line /games/metal.frame frame=metal-api-001\nappend-line /games/metal.frame queue=graphics\nappend-line /games/metal.frame present-mode=fifo\nappend-line /games/metal.frame completion=fire-and-forget\nappend-line /games/metal.frame metal-clear=001122ff\nappend-line /games/metal.frame metal-fill-rounded-rect=10,10,200,120,12,88aaffff\nappend-line /games/metal.frame metal-present-drawable=0,0,1024,768\nmkfile-path /games/vk.frame\nappend-line /games/vk.frame surface=640x480\nappend-line /games/vk.frame frame=vk-api-001\nappend-line /games/vk.frame queue=graphics\nappend-line /games/vk.frame present-mode=fifo\nappend-line /games/vk.frame completion=wait-present\nappend-line /games/vk.frame vk-cmd-clear-color=002244ff\nappend-line /games/vk.frame vk-cmd-fill-rect=50,50,200,100,ff8800ff\nappend-line /games/vk.frame vk-queue-present=0,0,640,480\nmkfile-path /games/web.frame\nappend-line /games/web.frame surface=640x360\nappend-line /games/web.frame frame=web-api-001\nappend-line /games/web.frame queue=graphics\nappend-line /games/web.frame present-mode=immediate\nappend-line /games/web.frame completion=fire-and-forget\nappend-line /games/web.frame webgpu-clear-pass=0f0f0fff\nappend-line /games/web.frame webgpu-copy-texture=backbuffer,0,0,640,360\nappend-line /games/web.frame webgpu-present=0,0,640,360\nmkfile-path /games/wgpu.frame\nappend-line /games/wgpu.frame surface=960x540\nappend-line /games/wgpu.frame frame=wgpu-api-001\nappend-line /games/wgpu.frame queue=graphics\nappend-line /games/wgpu.frame present-mode=fifo\nappend-line /games/wgpu.frame completion=wait-present\nappend-line /games/wgpu.frame wgpu-clear-pass=444444ff\nappend-line /games/wgpu.frame wgpu-fill-rect=0,0,960,540,123456ff\nappend-line /games/wgpu.frame wgpu-present=0,0,960,540\ngame-gfx-translate 77 /games/d3d9.frame\ngame-gfx-translate 78 /games/d3d10.frame\ngame-gfx-translate 79 /games/dx11.frame\ngame-gfx-translate 80 /games/dx12.frame\ngame-gfx-translate 81 /games/gl.frame\ngame-gfx-translate 82 /games/gles.frame\ngame-gfx-translate 83 /games/metal.frame\ngame-gfx-translate 84 /games/vk.frame\ngame-gfx-translate 85 /games/web.frame\ngame-gfx-translate 86 /games/wgpu.frame\ngame-gfx-next 77\ngame-gfx-next 78\ngame-gfx-next 79\ngame-gfx-next 80\ngame-gfx-next 81\ngame-gfx-next 82\ngame-gfx-next 83\ngame-gfx-next 84\ngame-gfx-next 85\ngame-gfx-next 86\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("pid=77 frame=d3d9-api-001"));
    assert!(stdout.contains("pid=78 frame=d3d10-api-001"));
    assert!(stdout.contains("pid=79 frame=dx11-api-001"));
    assert!(stdout.contains("pid=80 frame=dx12-api-001"));
    assert!(stdout.contains("pid=81 frame=gl-api-001"));
    assert!(stdout.contains("pid=82 frame=gles-api-001"));
    assert!(stdout.contains("pid=83 frame=metal-api-001"));
    assert!(stdout.contains("pid=84 frame=vk-api-001"));
    assert!(stdout.contains("pid=85 frame=web-api-001"));
    assert!(stdout.contains("pid=86 frame=wgpu-api-001"));
    assert!(stdout.contains(
        "game.gfx.next pid=77 frame=d3d9-api-001 api=direct3d9 translation=compat-to-vulkan"
    ));
    assert!(stdout.contains(
        "game.gfx.next pid=78 frame=d3d10-api-001 api=direct3d10 translation=compat-to-vulkan"
    ));
    assert!(stdout.contains(
        "game.gfx.next pid=79 frame=dx11-api-001 api=directx11 translation=compat-to-vulkan"
    ));
    assert!(stdout.contains(
        "game.gfx.next pid=80 frame=dx12-api-001 api=directx12 translation=compat-to-vulkan"
    ));
    assert!(
        stdout.contains(
            "game.gfx.next pid=81 frame=gl-api-001 api=opengl translation=compat-to-vulkan"
        )
    );
    assert!(stdout.contains(
        "game.gfx.next pid=82 frame=gles-api-001 api=opengles translation=compat-to-vulkan"
    ));
    assert!(stdout.contains(
        "game.gfx.next pid=83 frame=metal-api-001 api=metal translation=compat-to-vulkan"
    ));
    assert!(
        stdout
            .contains("game.gfx.next pid=84 frame=vk-api-001 api=vulkan translation=native-vulkan")
    );
    assert!(stdout.contains(
        "game.gfx.next pid=85 frame=web-api-001 api=webgpu translation=compat-to-vulkan"
    ));
    assert!(
        stdout.contains(
            "game.gfx.next pid=86 frame=wgpu-api-001 api=wgpu translation=compat-to-vulkan"
        )
    );
}
