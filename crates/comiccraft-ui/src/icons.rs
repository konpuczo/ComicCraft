use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioIcon {
    // App & Command Bar
    CspLogo,
    NewDoc,
    OpenFolder,
    SaveDisk,
    UndoArrow,
    RedoArrow,
    ClearTrash,
    FillBucket,
    TransformBox,
    DeselectBox,
    InvertSelection,
    ZoomOut,
    ZoomIn,
    FitScreen,
    FlipH,
    FlipV,
    MangaGuides,
    CanvasGrid,
    RulerBar,

    // Left Tool Strip (Authentic CSP Tool Palette Order)
    ToolZoom,
    ToolHand,
    ToolOperation,
    ToolMoveLayer,
    ToolMarquee,
    ToolEllipse,
    ToolLasso,
    ToolAutoSelect,
    ToolEyedropper,
    ToolPen,
    ToolPencil,
    ToolBrush,
    ToolAirbrush,
    ToolDecoration,
    ToolEraser,
    ToolBlend,
    ToolFill,
    ToolCloseAndFill,
    ToolGradient,
    ToolFigure,
    ToolComicFrame,
    ToolRuler,
    ToolBalloon,
    ToolCorrectLine,

    // Layer Property & Layer Palette Icons
    EyeOpen,
    EyeClosed,
    PadlockLocked,
    PadlockAlpha,
    ClipMaskBar,
    LighthouseRef,
    DraftPencil,
    BorderEffect,
    ToneGrid,
    LayerColorTint,
    NewRasterLayer,
    NewVectorLayer,
    NewLayerFolder,
    DuplicateLayer,
    MoveUp,
    MoveDown,
    MergeDown,
    LayerMask,
    DeleteTrash,
    HamburgerMenu,
}

/// Renders a crisp, anti-aliased monochrome vector icon inside `rect` matching Clip Studio Paint's dark UI.
pub fn draw_icon(painter: &egui::Painter, rect: Rect, icon: StudioIcon, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.5;
    let stroke = Stroke::new(1.35_f32, color);
    let thin = Stroke::new(1.0_f32, color);
    let thick = Stroke::new(1.8_f32, color);

    match icon {
        StudioIcon::CspLogo => {
            // Iconic Clip Studio overlapping S-waves / emblem inside rounded square
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.55));
            painter.rect_stroke(r, 2.5, stroke);
            painter.line_segment(
                [
                    Pos2::new(r.left() + 2.5, c.y - 2.0),
                    Pos2::new(r.right() - 2.5, c.y - 2.0),
                ],
                thin,
            );
            painter.line_segment(
                [
                    Pos2::new(r.left() + 2.5, c.y + 2.0),
                    Pos2::new(r.right() - 2.5, c.y + 2.0),
                ],
                thin,
            );
        }
        StudioIcon::HamburgerMenu => {
            let w = s * 1.1;
            for dy in [-3.5_f32, 0.0, 3.5] {
                painter.line_segment(
                    [Pos2::new(c.x - w, c.y + dy), Pos2::new(c.x + w, c.y + dy)],
                     thin,
                );
            }
        }
        StudioIcon::NewDoc | StudioIcon::NewRasterLayer => {
            let w = s * 1.1;
            let h = s * 1.4;
            let fold = s * 0.45;
            let tl = Pos2::new(c.x - w * 0.5, c.y - h * 0.5);
            let tr_pre = Pos2::new(c.x + w * 0.5 - fold, c.y - h * 0.5);
            let tr_post = Pos2::new(c.x + w * 0.5, c.y - h * 0.5 + fold);
            let br = Pos2::new(c.x + w * 0.5, c.y + h * 0.5);
            let bl = Pos2::new(c.x - w * 0.5, c.y + h * 0.5);
            painter.add(Shape::closed_line(
                vec![tl, tr_pre, tr_post, br, bl],
                stroke,
            ));
            painter.line_segment(
                [tr_pre, Pos2::new(tr_pre.x, tr_post.y)],
                thin,
            );
            painter.line_segment(
                [Pos2::new(tr_pre.x, tr_post.y), tr_post],
                thin,
            );
            if icon == StudioIcon::NewRasterLayer {
                let pc = Pos2::new(c.x - w * 0.35, c.y + h * 0.25);
                painter.line_segment([pc - Vec2::new(2.5, 0.0), pc + Vec2::new(2.5, 0.0)], stroke);
                painter.line_segment([pc - Vec2::new(0.0, 2.5), pc + Vec2::new(0.0, 2.5)], stroke);
            }
        }
        StudioIcon::NewVectorLayer => {
            draw_icon(painter, rect, StudioIcon::NewDoc, color);
            // Small vector cube badge inside
            let r = Rect::from_center_size(c + Vec2::new(0.0, 1.5), Vec2::splat(s * 0.7));
            painter.rect_stroke(r, 0.5, thin);
        }
        StudioIcon::OpenFolder | StudioIcon::NewLayerFolder => {
            let w = s * 1.4;
            let h = s * 1.05;
            let l = c.x - w * 0.5;
            let r = c.x + w * 0.5;
            let t = c.y - h * 0.45;
            let b = c.y + h * 0.55;
            let tab_r = l + w * 0.42;
            painter.add(Shape::closed_line(
                vec![
                    Pos2::new(l, t),
                    Pos2::new(tab_r - 1.5, t),
                    Pos2::new(tab_r + 1.0, t + 2.5),
                    Pos2::new(r, t + 2.5),
                    Pos2::new(r, b),
                    Pos2::new(l, b),
                ],
                stroke,
            ));
        }
        StudioIcon::SaveDisk => {
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.4));
            painter.rect_stroke(r, 1.5, stroke);
            let shutter = Rect::from_min_max(
                Pos2::new(r.left() + 2.5, r.top()),
                Pos2::new(r.right() - 2.5, r.top() + r.height() * 0.42),
            );
            painter.rect_stroke(shutter, 0.5, thin);
            let label = Rect::from_min_max(
                Pos2::new(r.left() + 2.0, r.bottom() - r.height() * 0.42),
                Pos2::new(r.right() - 2.0, r.bottom()),
            );
            painter.rect_filled(label, 0.5, color.gamma_multiply(0.35));
        }
        StudioIcon::UndoArrow => {
            let p_tip = Pos2::new(c.x - s * 0.65, c.y - s * 0.15);
            let p_mid = Pos2::new(c.x + s * 0.45, c.y - s * 0.15);
            let p_bot = Pos2::new(c.x + s * 0.45, c.y + s * 0.55);
            let p_end = Pos2::new(c.x - s * 0.25, c.y + s * 0.55);
            painter.add(Shape::line(vec![p_tip, p_mid, p_bot, p_end], stroke));
            painter.line_segment([p_tip, p_tip + Vec2::new(3.5, -3.2)], stroke);
            painter.line_segment([p_tip, p_tip + Vec2::new(3.5, 3.2)], stroke);
        }
        StudioIcon::RedoArrow => {
            let p_tip = Pos2::new(c.x + s * 0.65, c.y - s * 0.15);
            let p_mid = Pos2::new(c.x - s * 0.45, c.y - s * 0.15);
            let p_bot = Pos2::new(c.x - s * 0.45, c.y + s * 0.55);
            let p_end = Pos2::new(c.x + s * 0.25, c.y + s * 0.55);
            painter.add(Shape::line(vec![p_tip, p_mid, p_bot, p_end], stroke));
            painter.line_segment([p_tip, p_tip + Vec2::new(-3.5, -3.2)], stroke);
            painter.line_segment([p_tip, p_tip + Vec2::new(-3.5, 3.2)], stroke);
        }
        StudioIcon::ClearTrash | StudioIcon::DeleteTrash => {
            let w = s * 1.05;
            let h = s * 1.25;
            let top_y = c.y - h * 0.4;
            let bot_y = c.y + h * 0.55;
            // Lid
            painter.line_segment(
                [Pos2::new(c.x - w * 0.65, top_y), Pos2::new(c.x + w * 0.65, top_y)],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - w * 0.25, top_y - 2.0),
                    Pos2::new(c.x + w * 0.25, top_y - 2.0),
                ],
                stroke,
            );
            // Bin body
            painter.add(Shape::line(
                vec![
                    Pos2::new(c.x - w * 0.5, top_y),
                    Pos2::new(c.x - w * 0.4, bot_y),
                    Pos2::new(c.x + w * 0.4, bot_y),
                    Pos2::new(c.x + w * 0.5, top_y),
                ],
                stroke,
            ));
            // Vertical ribs
            painter.line_segment(
                [Pos2::new(c.x - 1.5, top_y + 2.5), Pos2::new(c.x - 1.5, bot_y - 2.0)],
                thin,
            );
            painter.line_segment(
                [Pos2::new(c.x + 1.5, top_y + 2.5), Pos2::new(c.x + 1.5, bot_y - 2.0)],
                thin,
            );
        }
        StudioIcon::FillBucket | StudioIcon::ToolFill | StudioIcon::ToolCloseAndFill | StudioIcon::LayerColorTint => {
            // Tilted paint bucket + ink drop
            let p1 = c + Vec2::new(-s * 0.15, -s * 0.65);
            let p2 = c + Vec2::new(s * 0.55, 0.0);
            let p3 = c + Vec2::new(-s * 0.05, s * 0.6);
            let p4 = c + Vec2::new(-s * 0.7, -s * 0.05);
            painter.add(Shape::closed_line(vec![p1, p2, p3, p4], stroke));
            // Pouring droplet on right
            let drop_c = c + Vec2::new(s * 0.65, s * 0.42);
            painter.circle_filled(drop_c, 2.0, color);
        }
        StudioIcon::TransformBox => {
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.25));
            painter.rect_stroke(r, 0.0, thin);
            for corner in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()] {
                painter.rect_filled(Rect::from_center_size(corner, Vec2::splat(3.0)), 0.0, color);
            }
        }
        StudioIcon::DeselectBox | StudioIcon::ToolMarquee => {
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.35));
            // Dashed rectangle
            let seg = 3.0;
            for x in [r.left(), r.right() - seg] {
                painter.line_segment([Pos2::new(x, r.top()), Pos2::new(x + seg, r.top())], stroke);
                painter.line_segment(
                    [Pos2::new(x, r.bottom()), Pos2::new(x + seg, r.bottom())],
                    stroke,
                );
            }
            for y in [r.top(), r.bottom() - seg] {
                painter.line_segment([Pos2::new(r.left(), y), Pos2::new(r.left(), y + seg)], stroke);
                painter.line_segment(
                    [Pos2::new(r.right(), y), Pos2::new(r.right(), y + seg)],
                    stroke,
                );
            }
            painter.line_segment(
                [Pos2::new(c.x - 1.5, r.top()), Pos2::new(c.x + 1.5, r.top())],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x - 1.5, r.bottom()), Pos2::new(c.x + 1.5, r.bottom())],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(r.left(), c.y - 1.5), Pos2::new(r.left(), c.y + 1.5)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(r.right(), c.y - 1.5), Pos2::new(r.right(), c.y + 1.5)],
                stroke,
            );
            if icon == StudioIcon::DeselectBox {
                painter.line_segment([c - Vec2::splat(2.5), c + Vec2::splat(2.5)], stroke);
                painter.line_segment(
                    [c + Vec2::new(-2.5, 2.5), c + Vec2::new(2.5, -2.5)],
                    stroke,
                );
            }
        }
        StudioIcon::ToolEllipse => {
            painter.circle_stroke(c, s * 0.65, stroke);
        }
        StudioIcon::ToolAutoSelect => {
            let tip = c + Vec2::new(s * 0.45, -s * 0.45);
            let base = c + Vec2::new(-s * 0.55, s * 0.55);
            painter.line_segment([base, tip], thick);
            painter.circle_filled(tip + Vec2::new(-s * 0.25, -s * 0.15), 1.2, color);
            painter.circle_filled(tip + Vec2::new(s * 0.15, s * 0.25), 1.0, color);
            painter.circle_filled(tip + Vec2::new(-s * 0.05, -s * 0.35), 1.0, color);
        }
        StudioIcon::InvertSelection => {
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.3));
            painter.rect_stroke(r, 1.0, stroke);
            let half = Rect::from_min_max(r.left_top(), Pos2::new(c.x, r.bottom()));
            painter.rect_filled(half, 0.0, color);
        }
        StudioIcon::ZoomOut | StudioIcon::ZoomIn | StudioIcon::ToolZoom => {
            let gc = c - Vec2::new(s * 0.15, s * 0.15);
            let rad = s * 0.55;
            painter.circle_stroke(gc, rad, stroke);
            painter.line_segment(
                [
                    gc + Vec2::splat(rad * 0.72),
                    c + Vec2::splat(s * 0.72),
                ],
                thick,
            );
            if icon == StudioIcon::ZoomIn {
                painter.line_segment([gc - Vec2::new(2.5, 0.0), gc + Vec2::new(2.5, 0.0)], thin);
                painter.line_segment([gc - Vec2::new(0.0, 2.5), gc + Vec2::new(0.0, 2.5)], thin);
            } else if icon == StudioIcon::ZoomOut {
                painter.line_segment([gc - Vec2::new(2.5, 0.0), gc + Vec2::new(2.5, 0.0)], thin);
            }
        }
        StudioIcon::FitScreen => {
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.35));
            let d = 3.5;
            // 4 corner brackets
            painter.add(Shape::line(
                vec![
                    r.left_top() + Vec2::new(0.0, d),
                    r.left_top(),
                    r.left_top() + Vec2::new(d, 0.0),
                ],
                stroke,
            ));
            painter.add(Shape::line(
                vec![
                    r.right_top() + Vec2::new(-d, 0.0),
                    r.right_top(),
                    r.right_top() + Vec2::new(0.0, d),
                ],
                stroke,
            ));
            painter.add(Shape::line(
                vec![
                    r.left_bottom() + Vec2::new(0.0, -d),
                    r.left_bottom(),
                    r.left_bottom() + Vec2::new(d, 0.0),
                ],
                stroke,
            ));
            painter.add(Shape::line(
                vec![
                    r.right_bottom() + Vec2::new(-d, 0.0),
                    r.right_bottom(),
                    r.right_bottom() + Vec2::new(0.0, -d),
                ],
                stroke,
            ));
        }
        StudioIcon::FlipH => {
            // Central mirror axis + left/right triangles
            painter.line_segment(
                [Pos2::new(c.x, c.y - s * 0.75), Pos2::new(c.x, c.y + s * 0.75)],
                thin,
            );
            painter.add(Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 2.0, c.y - s * 0.55),
                    Pos2::new(c.x - s * 0.75, c.y + s * 0.55),
                    Pos2::new(c.x - 2.0, c.y + s * 0.55),
                ],
                color,
                Stroke::NONE,
            ));
            painter.add(Shape::closed_line(
                vec![
                    Pos2::new(c.x + 2.0, c.y - s * 0.55),
                    Pos2::new(c.x + s * 0.75, c.y + s * 0.55),
                    Pos2::new(c.x + 2.0, c.y + s * 0.55),
                ],
                thin,
            ));
        }
        StudioIcon::FlipV => {
            painter.line_segment(
                [Pos2::new(c.x - s * 0.75, c.y), Pos2::new(c.x + s * 0.75, c.y)],
                thin,
            );
            painter.add(Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - s * 0.55, c.y - 2.0),
                    Pos2::new(c.x + s * 0.55, c.y - 2.0),
                    Pos2::new(c.x, c.y - s * 0.72),
                ],
                color,
                Stroke::NONE,
            ));
            painter.add(Shape::closed_line(
                vec![
                    Pos2::new(c.x - s * 0.55, c.y + 2.0),
                    Pos2::new(c.x + s * 0.55, c.y + 2.0),
                    Pos2::new(c.x, c.y + s * 0.72),
                ],
                thin,
            ));
        }
        StudioIcon::MangaGuides => {
            // Crop marks (Tonbo) icon
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.1));
            painter.rect_stroke(r, 0.0, thin);
            let ext = 2.8;
            for corner in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()] {
                painter.line_segment(
                    [corner - Vec2::new(ext, 0.0), corner + Vec2::new(ext, 0.0)],
                    thin,
                );
                painter.line_segment(
                    [corner - Vec2::new(0.0, ext), corner + Vec2::new(0.0, ext)],
                    thin,
                );
            }
        }
        StudioIcon::CanvasGrid => {
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.35));
            painter.rect_stroke(r, 0.5, thin);
            painter.line_segment([Pos2::new(c.x, r.top()), Pos2::new(c.x, r.bottom())], thin);
            painter.line_segment([Pos2::new(r.left(), c.y), Pos2::new(r.right(), c.y)], thin);
        }
        StudioIcon::RulerBar | StudioIcon::ToolRuler => {
            // Right-angle triangle ruler
            let p1 = Pos2::new(c.x - s * 0.65, c.y + s * 0.65);
            let p2 = Pos2::new(c.x + s * 0.65, c.y + s * 0.65);
            let p3 = Pos2::new(c.x - s * 0.65, c.y - s * 0.65);
            painter.add(Shape::closed_line(vec![p1, p2, p3], stroke));
            for i in 1..=3 {
                let x = p1.x + (i as f32 * 0.25) * (p2.x - p1.x);
                painter.line_segment([Pos2::new(x, p1.y), Pos2::new(x, p1.y - 2.5)], thin);
            }
        }
        StudioIcon::ToolHand => {
            // Stylized palm + 4 fingers
            let palm = Rect::from_min_max(
                Pos2::new(c.x - s * 0.45, c.y - s * 0.05),
                Pos2::new(c.x + s * 0.5, c.y + s * 0.65),
            );
            painter.rect_stroke(palm, 2.0, stroke);
            for fx in [-0.35_f32, -0.1, 0.15, 0.4] {
                let x = c.x + fx * s;
                painter.line_segment([Pos2::new(x, c.y - s * 0.05), Pos2::new(x, c.y - s * 0.68)], stroke);
            }
            painter.line_segment(
                [Pos2::new(c.x - s * 0.45, c.y + s * 0.2), Pos2::new(c.x - s * 0.72, c.y - s * 0.1)],
                stroke,
            );
        }
        StudioIcon::ToolOperation => {
            // 3D wireframe box + pointer arrow
            let r = Rect::from_center_size(c + Vec2::new(1.5, 1.5), Vec2::splat(s * 1.0));
            painter.rect_stroke(r, 0.5, thin);
            let p_tip = c - Vec2::new(s * 0.65, s * 0.65);
            painter.line_segment([p_tip, c + Vec2::new(-1.0, -1.0)], thick);
            painter.line_segment([p_tip, p_tip + Vec2::new(5.0, 1.0)], stroke);
            painter.line_segment([p_tip, p_tip + Vec2::new(1.0, 5.0)], stroke);
        }
        StudioIcon::ToolMoveLayer => {
            let len = s * 0.72;
            painter.line_segment([Pos2::new(c.x - len, c.y), Pos2::new(c.x + len, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x, c.y - len), Pos2::new(c.x, c.y + len)], stroke);
            // Arrowheads
            painter.line_segment([Pos2::new(c.x, c.y - len), Pos2::new(c.x - 2.5, c.y - len + 2.5)], thin);
            painter.line_segment([Pos2::new(c.x, c.y - len), Pos2::new(c.x + 2.5, c.y - len + 2.5)], thin);
            painter.line_segment([Pos2::new(c.x, c.y + len), Pos2::new(c.x - 2.5, c.y + len - 2.5)], thin);
            painter.line_segment([Pos2::new(c.x, c.y + len), Pos2::new(c.x + 2.5, c.y + len - 2.5)], thin);
        }
        StudioIcon::ToolLasso => {
            painter.circle_stroke(c - Vec2::new(0.0, 1.0), s * 0.58, stroke);
            painter.line_segment(
                [
                    c + Vec2::new(-s * 0.3, s * 0.45),
                    c + Vec2::new(-s * 0.65, s * 0.75),
                ],
                stroke,
            );
        }
        StudioIcon::ToolEyedropper => {
            let tip = c + Vec2::new(-s * 0.65, s * 0.65);
            let mid = c + Vec2::new(s * 0.15, -s * 0.15);
            let top = c + Vec2::new(s * 0.6, -s * 0.6);
            painter.line_segment([tip, mid], stroke);
            painter.line_segment([mid, top], Stroke::new(3.2_f32, color));
            painter.line_segment(
                [
                    mid + Vec2::new(-2.8, -2.8),
                    mid + Vec2::new(2.8, 2.8),
                ],
                stroke,
            );
        }
        StudioIcon::ToolPen => {
            // Iconic CSP Fountain/G-Pen Nib angled bottom-left
            let tip = c + Vec2::new(-s * 0.68, s * 0.68);
            let sh_l = c + Vec2::new(-s * 0.15, -s * 0.42);
            let sh_r = c + Vec2::new(s * 0.42, s * 0.15);
            let base = c + Vec2::new(s * 0.65, -s * 0.65);
            painter.add(Shape::closed_line(vec![tip, sh_l, base, sh_r], stroke));
            // Nib slit + breather hole
            let hole = c + Vec2::new(-s * 0.12, s * 0.12);
            painter.line_segment([tip, hole], thin);
            painter.circle_filled(hole, 1.3, color);
        }
        StudioIcon::ToolPencil => {
            // Sharpened hexagonal pencil
            let tip = c + Vec2::new(-s * 0.68, s * 0.68);
            let l1 = c + Vec2::new(-s * 0.45, s * 0.15);
            let l2 = c + Vec2::new(s * 0.35, -s * 0.65);
            let r2 = c + Vec2::new(s * 0.65, -s * 0.35);
            let r1 = c + Vec2::new(-s * 0.15, s * 0.45);
            painter.add(Shape::closed_line(vec![tip, l1, l2, r2, r1], stroke));
            painter.line_segment([l1, r1], thin);
        }
        StudioIcon::ToolBrush => {
            // Soft paintbrush tip + handle
            let tip = c + Vec2::new(-s * 0.72, s * 0.68);
            let bulb_l = c + Vec2::new(-s * 0.35, s * 0.05);
            let bulb_r = c + Vec2::new(s * 0.05, s * 0.45);
            let top = c + Vec2::new(s * 0.68, -s * 0.68);
            painter.add(Shape::closed_line(vec![tip, bulb_l, bulb_r], stroke));
            painter.line_segment([ (bulb_l + bulb_r.to_vec2()) * 0.5, top ], thick);
        }
        StudioIcon::ToolAirbrush => {
            // Spray nozzle + mist dots
            let noz = c + Vec2::new(s * 0.1, s * 0.1);
            let back = c + Vec2::new(s * 0.7, s * 0.55);
            painter.line_segment([noz, back], thick);
            for &(dx, dy) in &[(-0.45_f32, -0.15_f32), (-0.65, -0.35), (-0.35, -0.5), (-0.65, 0.05), (-0.2, -0.3)] {
                painter.circle_filled(c + Vec2::new(dx * s, dy * s), 1.1, color);
            }
        }
        StudioIcon::ToolDecoration => {
            // 4-point sparkle + mini star
            let r_long = s * 0.65;
            let r_short = s * 0.18;
            let pts = vec![
                Pos2::new(c.x, c.y - r_long),
                Pos2::new(c.x + r_short, c.y - r_short),
                Pos2::new(c.x + r_long, c.y),
                Pos2::new(c.x + r_short, c.y + r_short),
                Pos2::new(c.x, c.y + r_long),
                Pos2::new(c.x - r_short, c.y + r_short),
                Pos2::new(c.x - r_long, c.y),
                Pos2::new(c.x - r_short, c.y - r_short),
            ];
            painter.add(Shape::closed_line(pts, stroke));
        }
        StudioIcon::ToolEraser => {
            // Angled rectangular block eraser with paper sleeve
            let p1 = c + Vec2::new(-s * 0.7, s * 0.25);
            let p2 = c + Vec2::new(-s * 0.25, s * 0.7);
            let p3 = c + Vec2::new(s * 0.7, -s * 0.25);
            let p4 = c + Vec2::new(s * 0.25, -s * 0.7);
            painter.add(Shape::closed_line(vec![p1, p2, p3, p4], stroke));
            let m1 = p1 + (p4 - p1) * 0.42;
            let m2 = p2 + (p3 - p2) * 0.42;
            painter.line_segment([m1, m2], stroke);
        }
        StudioIcon::ToolBlend => {
            // Teardrop water blender
            let top = Pos2::new(c.x, c.y - s * 0.68);
            let bl = Pos2::new(c.x - s * 0.48, c.y + s * 0.25);
            let br = Pos2::new(c.x + s * 0.48, c.y + s * 0.25);
            let bot = Pos2::new(c.x, c.y + s * 0.68);
            painter.add(Shape::closed_line(vec![top, br, bot, bl], stroke));
        }
        StudioIcon::ToolGradient => {
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.35));
            painter.rect_stroke(r, 1.0, stroke);
            let h = r.height() / 3.0;
            painter.rect_filled(
                Rect::from_min_size(r.left_top(), Vec2::new(r.width(), h)),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_size(r.left_top() + Vec2::new(0.0, h), Vec2::new(r.width(), h)),
                0.0,
                color.gamma_multiply(0.45),
            );
        }
        StudioIcon::ToolFigure => {
            // Overlapping rectangle + circle
            let r = Rect::from_min_max(c - Vec2::splat(s * 0.65), c + Vec2::splat(s * 0.15));
            painter.rect_stroke(r, 0.5, stroke);
            painter.circle_stroke(c + Vec2::splat(s * 0.22), s * 0.45, stroke);
        }
        StudioIcon::ToolComicFrame => {
            // Manga page divided into 3 asymmetric comic panels
            let r = Rect::from_center_size(c, Vec2::splat(s * 1.38));
            painter.rect_stroke(r, 0.5, stroke);
            painter.line_segment(
                [
                    Pos2::new(r.left(), c.y - 1.0),
                    Pos2::new(r.right(), c.y + 1.5),
                ],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x, c.y + 0.2), Pos2::new(c.x - 1.5, r.bottom())],
                stroke,
            );
        }
        StudioIcon::ToolBalloon => {
            let r = Rect::from_center_size(c - Vec2::new(0.0, 1.5), Vec2::new(s * 1.4, s * 1.05));
            painter.rect_stroke(r, 4.5, stroke);
            // Speech tail
            painter.line_segment(
                [
                    Pos2::new(c.x - 2.0, r.bottom()),
                    Pos2::new(c.x - 4.0, r.bottom() + 3.5),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + 2.0, r.bottom()),
                    Pos2::new(c.x - 4.0, r.bottom() + 3.5),
                ],
                stroke,
            );
        }
        StudioIcon::ToolCorrectLine => {
            // Curve with control vertex square
            let p1 = c + Vec2::new(-s * 0.65, s * 0.5);
            let p2 = c;
            let p3 = c + Vec2::new(s * 0.65, -s * 0.5);
            painter.add(Shape::line(vec![p1, p2, p3], stroke));
            painter.rect_filled(Rect::from_center_size(p2, Vec2::splat(3.8)), 0.0, color);
        }
        StudioIcon::EyeOpen => {
            let rx = s * 0.72;
            let ry = s * 0.42;
            let pts = vec![
                Pos2::new(c.x - rx, c.y),
                Pos2::new(c.x - rx * 0.45, c.y - ry),
                Pos2::new(c.x + rx * 0.45, c.y - ry),
                Pos2::new(c.x + rx, c.y),
                Pos2::new(c.x + rx * 0.45, c.y + ry),
                Pos2::new(c.x - rx * 0.45, c.y + ry),
            ];
            painter.add(Shape::closed_line(pts, stroke));
            painter.circle_filled(c, s * 0.24, color);
        }
        StudioIcon::EyeClosed => {
            let rx = s * 0.65;
            painter.line_segment([Pos2::new(c.x - rx, c.y), Pos2::new(c.x + rx, c.y)], thin);
        }
        StudioIcon::PadlockLocked | StudioIcon::PadlockAlpha => {
            let body = Rect::from_min_max(
                Pos2::new(c.x - s * 0.5, c.y - s * 0.05),
                Pos2::new(c.x + s * 0.5, c.y + s * 0.65),
            );
            painter.rect_stroke(body, 1.0, stroke);
            let shackle = Rect::from_min_max(
                Pos2::new(c.x - s * 0.3, c.y - s * 0.6),
                Pos2::new(c.x + s * 0.3, c.y - s * 0.05),
            );
            painter.rect_stroke(shackle, 2.5, thin);
            if icon == StudioIcon::PadlockAlpha {
                // Checkerboard dots at top-left
                painter.rect_filled(
                    Rect::from_min_size(c - Vec2::splat(s * 0.75), Vec2::splat(2.5)),
                    0.0,
                    color,
                );
            }
        }
        StudioIcon::ClipMaskBar => {
            // Iconic CSP Clip at Layer Below icon (two stacked squares + left red bar)
            let r1 = Rect::from_min_size(c - Vec2::new(s * 0.3, s * 0.6), Vec2::splat(s * 0.6));
            let r2 = Rect::from_min_size(c + Vec2::new(-s * 0.3, 0.5), Vec2::splat(s * 0.6));
            painter.rect_stroke(r1, 0.5, thin);
            painter.rect_filled(r2, 0.5, color);
            painter.line_segment(
                [
                    Pos2::new(c.x - s * 0.65, c.y - s * 0.6),
                    Pos2::new(c.x - s * 0.65, c.y + s * 0.6),
                ],
                Stroke::new(2.0_f32, Color32::from_rgb(235, 65, 65)),
            );
        }
        StudioIcon::LighthouseRef => {
            // Iconic CSP Reference Layer Lighthouse!
            let base_l = Pos2::new(c.x - s * 0.4, c.y + s * 0.68);
            let base_r = Pos2::new(c.x + s * 0.4, c.y + s * 0.68);
            let top_l = Pos2::new(c.x - s * 0.2, c.y - s * 0.35);
            let top_r = Pos2::new(c.x + s * 0.2, c.y - s * 0.35);
            painter.add(Shape::closed_line(vec![base_l, top_l, top_r, base_r], stroke));
            // Beacon light rays
            let lamp = Pos2::new(c.x, c.y - s * 0.5);
            painter.circle_filled(lamp, 1.6, color);
            painter.line_segment(
                [lamp + Vec2::new(-2.5, 0.0), lamp + Vec2::new(-s * 0.68, -2.0)],
                thin,
            );
            painter.line_segment(
                [lamp + Vec2::new(2.5, 0.0), lamp + Vec2::new(s * 0.68, -2.0)],
                thin,
            );
        }
        StudioIcon::DraftPencil => {
            draw_icon(painter, rect, StudioIcon::ToolPencil, color);
            painter.line_segment(
                [
                    Pos2::new(c.x + s * 0.2, c.y - s * 0.68),
                    Pos2::new(c.x + s * 0.2, c.y + s * 0.68),
                ],
                Stroke::new(1.5_f32, Color32::from_rgb(55, 165, 245)),
            );
        }
        StudioIcon::BorderEffect => {
            painter.circle_stroke(c, s * 0.65, stroke);
            painter.circle_stroke(c, s * 0.32, thin);
        }
        StudioIcon::ToneGrid => {
            for dy in [-0.35_f32, 0.35] {
                for dx in [-0.35_f32, 0.35] {
                    painter.circle_filled(c + Vec2::new(dx * s, dy * s), 1.9, color);
                }
            }
        }
        StudioIcon::DuplicateLayer => {
            let r1 = Rect::from_center_size(c - Vec2::splat(1.8), Vec2::splat(s * 1.0));
            let r2 = Rect::from_center_size(c + Vec2::splat(1.8), Vec2::splat(s * 1.0));
            painter.rect_stroke(r1, 0.5, thin);
            painter.rect_stroke(r2, 0.5, stroke);
        }
        StudioIcon::MoveUp => {
            painter.add(Shape::closed_line(
                vec![
                    Pos2::new(c.x, c.y - s * 0.45),
                    Pos2::new(c.x + s * 0.5, c.y + s * 0.35),
                    Pos2::new(c.x - s * 0.5, c.y + s * 0.35),
                ],
                stroke,
            ));
        }
        StudioIcon::MoveDown => {
            painter.add(Shape::closed_line(
                vec![
                    Pos2::new(c.x - s * 0.5, c.y - s * 0.35),
                    Pos2::new(c.x + s * 0.5, c.y - s * 0.35),
                    Pos2::new(c.x, c.y + s * 0.45),
                ],
                stroke,
            ));
        }
        StudioIcon::MergeDown => {
            let top_y = c.y - s * 0.55;
            let mid_y = c.y + s * 0.2;
            let bot_y = c.y + s * 0.6;
            painter.line_segment([Pos2::new(c.x, top_y), Pos2::new(c.x, mid_y)], stroke);
            painter.line_segment(
                [Pos2::new(c.x, mid_y), Pos2::new(c.x - 3.0, mid_y - 3.0)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x, mid_y), Pos2::new(c.x + 3.0, mid_y - 3.0)],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - s * 0.55, bot_y),
                    Pos2::new(c.x + s * 0.55, bot_y),
                ],
                thick,
            );
        }
        StudioIcon::LayerMask => {
            let r = Rect::from_center_size(c, Vec2::new(s * 1.35, s * 1.05));
            painter.rect_stroke(r, 1.0, stroke);
            painter.circle_filled(c, s * 0.32, color);
        }
    }
}

/// Interactive square icon button used in CSP Command Bar, Tool Strip, Layer Property, and Layer Action Bar.
pub fn icon_button(
    ui: &mut Ui,
    icon: StudioIcon,
    size: Vec2,
    selected: bool,
    tooltip: &str,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter_at(rect);

    let bg = if selected {
        Color32::from_rgb(38, 108, 212) // CSP active Royal Blue
    } else if resp.hovered() {
        Color32::from_rgb(62, 66, 74)
    } else {
        Color32::TRANSPARENT
    };

    if bg != Color32::TRANSPARENT {
        painter.rect_filled(rect, 2.0, bg);
    }
    if selected {
        painter.rect_stroke(
            rect,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgb(80, 150, 250)),
        );
    }

    let fg = if selected || resp.hovered() {
        Color32::WHITE
    } else {
        Color32::from_rgb(208, 212, 218)
    };

    draw_icon(&painter, rect.shrink(2.0), icon, fg);
    resp.on_hover_text(tooltip)
}

/// Draws a live tapered S-curve brush stroke preview inside `rect` (used in CSP's Sub Tool list).
pub fn draw_stroke_preview(painter: &egui::Painter, rect: Rect, thickness: f32, color: Color32) {
    let steps = 20;
    let max_r = (thickness * 0.22).clamp(1.0, 5.5);
    let w = rect.width() - 8.0;
    let x0 = rect.left() + 4.0;
    let cy = rect.center().y;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = x0 + t * w;
        let y = cy + (t * std::f32::consts::TAU).sin() * (rect.height() * 0.24);
        // Taper at both ends like a real G-Pen stroke
        let env = (t * std::f32::consts::PI).sin();
        let r = (max_r * env).max(0.6);
        painter.circle_filled(Pos2::new(x, y), r, color);
    }
}

/// Clip Studio Paint's signature inline bar slider (`Tool Property` & `Layer Opacity`).
pub fn csp_bar_slider(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    logarithmic: bool,
    decimals: usize,
    suffix: &str,
) -> bool {
    csp_bar_slider_reserved(ui, 0.0, label, value, range, logarithmic, decimals, suffix)
}

/// Inline CSP bar slider that reserves `reserve_right` pixels for trailing buttons on the same row.
pub fn csp_bar_slider_reserved(
    ui: &mut Ui,
    reserve_right: f32,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    logarithmic: bool,
    decimals: usize,
    suffix: &str,
) -> bool {
    let min = *range.start();
    let max = *range.end();
    let w = (ui.available_width() - reserve_right).max(50.0);
    let desired = Vec2::new(w, 20.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    let mut changed = false;
    if let Some(ptr) = resp.interact_pointer_pos() {
        let raw_t = ((ptr.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
        let new_val = if logarithmic && min > 0.0 {
            min * (max / min).powf(raw_t)
        } else {
            min + raw_t * (max - min)
        };
        if (*value - new_val).abs() > 1e-4 {
            *value = new_val;
            changed = true;
        }
    }

    let norm_t = if logarithmic && min > 0.0 {
        ((*value / min).ln() / (max / min).ln()).clamp(0.0, 1.0)
    } else {
        ((*value - min) / (max - min).max(1e-5)).clamp(0.0, 1.0)
    };

    // Dark recessed trough
    painter.rect_filled(rect, 1.5, Color32::from_rgb(27, 29, 32));

    // CSP Blue progress fill bar
    let fill_w = (rect.width() * norm_t).max(2.0);
    let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
    let bar_col = if resp.hovered() || resp.dragged() {
        Color32::from_rgb(56, 118, 208)
    } else {
        Color32::from_rgb(42, 92, 168)
    };
    painter.rect_filled(fill_rect, 1.5, bar_col);
    painter.rect_stroke(rect, 1.5, Stroke::new(1.0_f32, Color32::from_rgb(54, 58, 64)));

    // Left label
    painter.text(
        rect.left_center() + Vec2::new(6.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        Color32::from_rgb(232, 236, 242),
    );

    // Right numeric value
    let val_str = format!("{:.*}{}", decimals, *value, suffix);
    painter.text(
        rect.right_center() - Vec2::new(6.0, 0.0),
        Align2::RIGHT_CENTER,
        val_str,
        FontId::monospace(11.0),
        Color32::WHITE,
    );

    changed
}

/// Renders an authentic 21px Clip Studio Paint dock palette tab bar with hamburger menu (`≡`).
pub fn csp_palette_tabs<T: Copy + PartialEq>(
    ui: &mut Ui,
    current: &mut T,
    tabs: &[(T, &str)],
) {
    let width = ui.available_width();
    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(width, 21.0), Sense::hover());
    let painter = ui.painter_at(bar_rect);

    // Dark header trough
    painter.rect_filled(bar_rect, 0.0, Color32::from_rgb(31, 33, 37));
    painter.line_segment(
        [bar_rect.left_bottom(), bar_rect.right_bottom()],
        Stroke::new(1.0_f32, Color32::from_rgb(56, 60, 66)),
    );

    // Left hamburger icon
    let menu_rect = Rect::from_min_size(bar_rect.left_top(), Vec2::new(20.0, 21.0));
    draw_icon(
        &painter,
        menu_rect.shrink(3.0),
        StudioIcon::HamburgerMenu,
        Color32::from_rgb(165, 170, 178),
    );

    let mut x = bar_rect.left() + 21.0;
    let avail_tab_w = ((width - 24.0) / tabs.len().max(1) as f32).clamp(48.0, 125.0);

    for &(val, title) in tabs {
        let is_active = *current == val;
        let t_rect = Rect::from_min_size(Pos2::new(x, bar_rect.top() + 1.0), Vec2::new(avail_tab_w - 1.0, 20.0));
        let resp = ui.interact(t_rect, ui.id().with(title), Sense::click());

        let bg = if is_active {
            Color32::from_rgb(52, 55, 61)
        } else if resp.hovered() {
            Color32::from_rgb(42, 45, 50)
        } else {
            Color32::from_rgb(35, 37, 42)
        };
        painter.rect_filled(
            t_rect,
            Rounding {
                nw: 2.0,
                ne: 2.0,
                sw: 0.0,
                se: 0.0,
            },
            bg,
        );
        if is_active {
            // Subtle top highlight stripe on active palette tab
            painter.line_segment(
                [t_rect.left_top(), t_rect.right_top()],
                Stroke::new(1.5_f32, Color32::from_rgb(65, 135, 235)),
            );
        }

        let text_col = if is_active {
            Color32::from_rgb(238, 242, 248)
        } else {
            Color32::from_rgb(165, 170, 178)
        };
        painter.text(
            t_rect.center(),
            Align2::CENTER_CENTER,
            title,
            FontId::proportional(11.0),
            text_col,
        );

        if resp.clicked() {
            *current = val;
        }
        x += avail_tab_w;
    }
}
