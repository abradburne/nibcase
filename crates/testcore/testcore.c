/* Original deterministic libretro fixture. No emulator implementation or game is included. */
#include "libretro.h"
#include <string.h>
#ifdef NIBCASE_TEST_STDOUT
#include <stdio.h>
#endif
static retro_environment_t env;
static retro_video_refresh_t video;
static retro_audio_sample_t sample;
static retro_audio_sample_batch_t batch;
static retro_input_poll_t poll;
static retro_input_state_t input;
static unsigned frame, format, mode;
static unsigned char sram[16], rtc[8];
/* The console's work RAM, for RetroAchievements: [0] the frame, [1] the A button. */
static unsigned char wram[64];
static uint32_t pixels[8];
static unsigned char content[4];
static const struct retro_core_option_v2_category categories[] = {
    {"video", "Video", "Test video options"}, {0}
};
static const struct retro_core_option_v2_definition options[] = {
    {.key="fixture_format", .desc="Pixel format", .category_key="video",
        .values={{"1555", "0RGB1555"}, {"8888", "XRGB8888"}, {"565", "RGB565"}}, .default_value="565"},
    {.key="fixture_hidden", .desc="A hidden option", .values={{"off", 0}, {"on", 0}}, .default_value="off"},
    {0}
};
static bool display(void) {
    struct retro_variable v = {"fixture_format", 0};
    env(RETRO_ENVIRONMENT_GET_VARIABLE, &v);
    struct retro_core_option_display d = {"fixture_hidden", v.value && !strcmp(v.value, "8888")};
    return env(RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY, &d);
}
unsigned retro_api_version(void) { return RETRO_API_VERSION; }
void retro_set_environment(retro_environment_t cb) {
    env=cb;
    struct retro_core_options_v2 defs = {(void *)categories, (void *)options};
    env(RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2, &defs);
    struct retro_core_options_update_display_callback update={display};
    env(RETRO_ENVIRONMENT_SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK, &update);
    display();
}
void retro_set_video_refresh(retro_video_refresh_t cb) { video=cb; }
void retro_set_audio_sample(retro_audio_sample_t cb) { sample=cb; }
void retro_set_audio_sample_batch(retro_audio_sample_batch_t cb) { batch=cb; }
void retro_set_input_poll(retro_input_poll_t cb) { poll=cb; }
void retro_set_input_state(retro_input_state_t cb) { input=cb; }
/* The first port's device, echoed in sram[10] (low byte). */
static unsigned port0;
void retro_set_controller_port_device(unsigned p,unsigned d) { if(p==0) port0=d; }
void retro_get_system_info(struct retro_system_info *i) {
    *i=(struct retro_system_info){"nibcase test core", "1", "sfc|smc", false, false};
}
void retro_get_system_av_info(struct retro_system_av_info *i) {
    *i=(struct retro_system_av_info){{2,2,4,4,1.0f},{60.0,32040.5}};
}
void retro_init(void) {
#ifdef NIBCASE_TEST_STDOUT
    printf("fixture core stdout diagnostic\n");
#endif
    struct retro_log_callback log={0};
    if(env(RETRO_ENVIRONMENT_GET_LOG_INTERFACE,&log)) log.log(RETRO_LOG_INFO,"fixture log %d\n",7);
    const char *dir=0;
    if(!env(RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY,&dir) || !dir) env(RETRO_ENVIRONMENT_SHUTDOWN,0);
}
void retro_deinit(void) {}
/* Mode 10: a game of two discs, swapped through the disk control interface (the tray must be
   open to change disc); the disc in the drive is echoed in sram[9]. */
static unsigned disc; static bool tray_open;
static bool set_eject(bool ejected) { tray_open=ejected; return true; }
static bool get_eject(void) { return tray_open; }
static unsigned get_index(void) { return disc; }
/* Index 2 is an empty drive, as libretro.h allows. */
static bool set_index(unsigned i) { if(!tray_open || i>2) return false; disc=i; return true; }
static unsigned get_count(void) { return 2; }
static bool get_label(unsigned i, char *label, size_t len) {
    if(i>1 || !label || len<9) return false;
    strcpy(label, i ? "Disc Two" : "Disc One"); return true;
}
bool retro_load_game(const struct retro_game_info *g) {
    if(!g || g->size<4 || !g->data) return false;
    memcpy(content,g->data,4); mode=content[0]; frame=0; memset(sram,0,sizeof(sram));
    struct retro_input_descriptor inputs[] = {
        {0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT, "Move left"},
        {0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_A, "Jump"},
        {1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_B, "Player 2"},
        {0},
    };
    env(RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS, inputs);
    /* Two pads for the first port, as PlayStation cores offer: digital and analog. */
    static const struct retro_controller_description pads[] = {
        {"Standard", RETRO_DEVICE_JOYPAD},
        {"DualShock", RETRO_DEVICE_SUBCLASS(RETRO_DEVICE_ANALOG, 1)},
    };
    static const struct retro_controller_info ports[] = {{pads, 2}, {0, 0}};
    env(RETRO_ENVIRONMENT_SET_CONTROLLER_INFO, (void *)ports);
    if(mode==10) {
        struct retro_disk_control_ext_callback d={0};
        d.set_eject_state=set_eject; d.get_eject_state=get_eject; d.get_image_index=get_index;
        d.set_image_index=set_index; d.get_num_images=get_count; d.get_image_label=get_label;
        disc=0; tray_open=false;
        env(RETRO_ENVIRONMENT_SET_DISK_CONTROL_EXT_INTERFACE,&d);
    }
    if(mode==4) { uint64_t q=RETRO_SERIALIZATION_QUIRK_SINGLE_SESSION; env(RETRO_ENVIRONMENT_SET_SERIALIZATION_QUIRKS,&q); }
    /* Mode 11 maps its work RAM where a Super Nintendo's is, $7E0000 (and its mirror at
       $7F0000, which the select bits leave out), as a core with a memory map does. */
    memset(wram,0,sizeof(wram));
    if(mode==11) {
        static struct retro_memory_descriptor d[1];
        d[0]=(struct retro_memory_descriptor){.flags=RETRO_MEMDESC_SYSTEM_RAM, .ptr=wram,
            .start=0x7E0000, .select=0xFE0000, .len=sizeof(wram)};
        struct retro_memory_map map={d, 1};
        env(RETRO_ENVIRONMENT_SET_MEMORY_MAPS,&map);
    }
    return true;
}
void retro_unload_game(void) {}
void retro_reset(void) { frame=0; }
void retro_run(void) {
    if(mode==5 && frame==1) {
        struct retro_core_options_v2 defs = {(void *)categories, (void *)options};
        env(RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2, &defs);
    }
    struct retro_variable v={"fixture_format",0}; env(RETRO_ENVIRONMENT_GET_VARIABLE,&v);
    format=v.value && !strcmp(v.value,"8888") ? RETRO_PIXEL_FORMAT_XRGB8888 :
        v.value && !strcmp(v.value,"1555") ? RETRO_PIXEL_FORMAT_0RGB1555 : RETRO_PIXEL_FORMAT_RGB565;
    env(RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,&format);
    poll();
    sram[0]=(unsigned char)input(0,RETRO_DEVICE_JOYPAD,0,RETRO_DEVICE_ID_JOYPAD_A);
    sram[1]=(unsigned char)input(0,RETRO_DEVICE_JOYPAD,0,RETRO_DEVICE_ID_JOYPAD_B);
    sram[2]=(unsigned char)input(0,RETRO_DEVICE_JOYPAD,0,RETRO_DEVICE_ID_JOYPAD_MASK);
    sram[3]=(unsigned char)frame;
    sram[9]=(unsigned char)disc;
    sram[10]=(unsigned char)port0;
    wram[0]=(unsigned char)frame;
    wram[1]=sram[0];
    /* Mode 8 echoes the pointer: x and y (little-endian) and whether it is pressed. */
    if(mode==8) {
        /* Read on port 2, as gw-libretro does: the pointer isn't a player's. */
        int16_t x=input(2,RETRO_DEVICE_POINTER,0,RETRO_DEVICE_ID_POINTER_X);
        int16_t y=input(2,RETRO_DEVICE_POINTER,0,RETRO_DEVICE_ID_POINTER_Y);
        memcpy(sram+4,&x,2); memcpy(sram+6,&y,2);
        sram[8]=(unsigned char)input(2,RETRO_DEVICE_POINTER,0,RETRO_DEVICE_ID_POINTER_PRESSED);
    }
    if(mode==3) { video((void *)-1,2,2,8); return; }
    if((mode==2 && frame==1) || mode==7) {
        if(mode==7) { unsigned r=1; env(RETRO_ENVIRONMENT_SET_ROTATION,&r); }
        struct retro_game_geometry g={3,2,4,4,1.5f}; env(RETRO_ENVIRONMENT_SET_GEOMETRY,&g);
        pixels[0]=0xFF0000; pixels[1]=0x00FF00; pixels[2]=0x0000FF;
        pixels[4]=0xFF0000; pixels[5]=0x00FF00; pixels[6]=0x0000FF;
        unsigned f=RETRO_PIXEL_FORMAT_XRGB8888; env(RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,&f);
        video(pixels,3,2,16);
    } else if(frame && frame%2) { video(0,2,2,8); }
    else {
        memset(pixels,0xCC,sizeof(pixels));
        if(format==RETRO_PIXEL_FORMAT_XRGB8888) { pixels[0]=0xFF0000; pixels[1]=0x0000FF; pixels[4]=0x00FF00; pixels[5]=0xFFFFFF; }
        else { uint16_t *p=(void *)pixels; p[0]=format==RETRO_PIXEL_FORMAT_RGB565?0xF800:0x7C00; p[1]=31; p[4]=format==RETRO_PIXEL_FORMAT_RGB565?0x7E0:0x3E0; p[5]=0xFFFF; }
        video(pixels,2,2,format==RETRO_PIXEL_FORMAT_XRGB8888?16:8);
    }
    int16_t sound[8]={(int16_t)frame,-(int16_t)frame,100,-100,200,-200,300,-300};
    batch(sound,4); sample(400,-400); frame++;
}
/* Mode 9 has no save states, as gw-libretro hasn't. */
size_t retro_serialize_size(void) { return mode==9 ? 0 : 4; }
bool retro_serialize(void *p,size_t n) { if(n!=4)return false; memcpy(p,&frame,4); return true; }
bool retro_unserialize(const void *p,size_t n) { if(n!=4)return false; memcpy(&frame,p,4); return true; }
/* Mode 6 shows no battery RAM until a frame has run, as some cores do. */
static int hidden(void) { return mode==6 && frame==0; }
void *retro_get_memory_data(unsigned id) { return hidden()?0:id==RETRO_MEMORY_SAVE_RAM?sram:id==RETRO_MEMORY_RTC?rtc:id==RETRO_MEMORY_SYSTEM_RAM?wram:0; }
size_t retro_get_memory_size(unsigned id) { return hidden()?0:id==RETRO_MEMORY_SAVE_RAM?sizeof(sram):id==RETRO_MEMORY_RTC?sizeof(rtc):id==RETRO_MEMORY_SYSTEM_RAM?sizeof(wram):0; }
