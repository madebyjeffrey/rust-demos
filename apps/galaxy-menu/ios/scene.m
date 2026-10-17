// winit 0.30 owns the event loop and creates its UIWindow. iOS 27 additionally
// requires a scene lifecycle. Attach that same window instead of creating a
// second renderer or view. Remove this bridge when winit supports UIScene.
#import <UIKit/UIKit.h>

@interface GalaxySceneDelegate : UIResponder <UIWindowSceneDelegate>
@property(nonatomic, strong) UIWindow *window;
@property(nonatomic, strong) UIUpdateLink *updateLink;
@end

static __weak UIWindowScene *galaxyScene;
static __weak UIWindow *galaxyWindow;

static void attachWindow(UIWindow *window, UIWindowScene *scene) {
    if (!window || !scene || ![window isKindOfClass:NSClassFromString(@"WinitUIWindow")]) {
        return;
    }
    if (window.windowScene != scene) {
        window.windowScene = scene;
        window.frame = scene.effectiveGeometry.coordinateSpace.bounds;
        ((GalaxySceneDelegate *)scene.delegate).window = window;
        [window makeKeyAndVisible];
    }
}

@implementation GalaxySceneDelegate
- (void)scene:(UIScene *)scene willConnectToSession:(UISceneSession *)session
      options:(UISceneConnectionOptions *)options {
    if (![scene isKindOfClass:UIWindowScene.class]) {
        return;
    }
    galaxyScene = (UIWindowScene *)scene;
    attachWindow(galaxyWindow, galaxyScene);
    // UIUpdateLink can express UI update preferences without owning rendering
    // callbacks. Request display-aligned 60 Hz updates for winit's existing loop.
    self.updateLink.enabled = NO;
    self.updateLink = [UIUpdateLink updateLinkForWindowScene:galaxyScene];
    self.updateLink.enabled = NO;
    self.updateLink.requiresContinuousUpdates = YES;
    self.updateLink.preferredFrameRateRange = CAFrameRateRangeMake(60, 60, 60);
    self.updateLink.enabled = scene.activationState == UISceneActivationStateForegroundActive;
}
- (void)sceneDidBecomeActive:(UIScene *)scene {
    if (scene == galaxyScene) {
        self.updateLink.enabled = YES;
    }
}
- (void)sceneWillResignActive:(UIScene *)scene {
    if (scene == galaxyScene) {
        self.updateLink.enabled = NO;
    }
}
// Scene geometry, rather than the device screen, changes for both rotation
// and iPad multitasking. Resizing winit's existing window lets its view emit
// the normal resize event and keeps the renderer covering the entire window.
- (void)windowScene:(UIWindowScene *)scene
        didUpdateEffectiveGeometry:(UIWindowSceneGeometry *)previousGeometry {
    if (scene == galaxyScene && self.window) {
        self.window.frame = scene.effectiveGeometry.coordinateSpace.bounds;
        [self.window setNeedsLayout];
        [self.window layoutIfNeeded];
    }
}
- (void)sceneDidDisconnect:(UIScene *)scene {
    if (galaxyScene == scene) {
        // UIUpdateLink has no invalidate method. Disabling removes its effect;
        // releasing the scene-owned link ends this scene's timing request.
        self.updateLink.enabled = NO;
        self.updateLink = nil;
        self.window = nil;
        galaxyScene = nil;
    }
}
@end

// Called on the main thread before Bevy enters winit's UIApplicationMain.
void galaxy_register_scene(void) {
    [NSNotificationCenter.defaultCenter addObserverForName:UIWindowDidBecomeVisibleNotification
        object:nil queue:nil usingBlock:^(NSNotification *notification) {
            UIWindow *window = notification.object;
            if ([window isKindOfClass:NSClassFromString(@"WinitUIWindow")]) {
                galaxyWindow = window;
                attachWindow(window, galaxyScene);
            }
        }];
}

// Read the render view's insets in UIKit points (Bevy logical window units).
// Poll on the main thread so safe-area-only changes are picked up even when
// the window dimensions stay the same. No model-specific device constants.
bool galaxy_window_safe_area(float *insets) {
    UIView *view = galaxyWindow.rootViewController.view;
    if (!view || !view.window) {
        return false;
    }
    UIEdgeInsets safe = view.safeAreaInsets;
    insets[0] = (float)safe.top;
    insets[1] = (float)safe.right;
    insets[2] = (float)safe.bottom;
    insets[3] = (float)safe.left;
    return true;
}
