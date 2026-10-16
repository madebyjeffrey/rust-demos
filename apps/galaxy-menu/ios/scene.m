// winit 0.30 owns the event loop and creates its UIWindow. iOS 27 additionally
// requires a scene lifecycle. Attach that same window instead of creating a
// second renderer or view. Remove this bridge when winit supports UIScene.
#import <UIKit/UIKit.h>

@interface GalaxySceneDelegate : UIResponder <UIWindowSceneDelegate>
@property(nonatomic, strong) UIWindow *window;
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
}
- (void)sceneDidDisconnect:(UIScene *)scene {
    if (galaxyScene == scene) {
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
