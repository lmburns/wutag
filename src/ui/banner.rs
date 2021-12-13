//! Create a header/banner to fill up space in the help menu within the TUI

// Credit: idea and outline came from `orhun/gpg-tui`
//  * Using his work to help me learn how to code a TUI

// Need to get better art

use tui::layout::Rect;

pub(crate) const BANNERS: &[&str] = &[
    env!("CARGO_PKG_NAME"),
    r#"MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
WWMMMMWWWWWWMMWWWMWWWWWWWWMMWWWWWMMMWWWWWWW
;,WWWWN,<d,lWWk,<W',,,,,,dWWN,,'WWMWN>,..,0
; O0o0O <> 'WWb ,W00o .00XMW< , bWWX  >KKdO
x o, ,o x> 'WWb ,WWWk .WWWWX .N  NWo .WObbk
N , x , Nl 'WWb ,WMWk .WMMW'  ,  <Wd  Wx; ,
W. ,M, .W0  ;>. bWMWk .WMWO .KXK  XW' .>; ,
WkbKMKbkWWXdllbKWMMWXbxWMWOb0WWWkb0WWKbllxN
MWWWMWWWMMWWWWWWMMMMWWWMMMWWWMMMWWWMMWWWWWW
"#,
    r#"MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
WWMMMMMWWWWWWMMMWWWMWWWWWWWWMMMWWWWWMMMMWWWWWWW
o>XWWWW0>dO>oWMWo>0W>>>>>>>>NWWWll>NWMMWWkl;;>O
' bWNNW> >> .WMW. lW<<;  ;<<NWWb   <WMWX. .>l;'
d ;O  K, k> .WMW. lWWWK  0WWMWN  0  KWW' .WWWMW
X ,;. ;. N> .WMW. lMMWK  0WMMW< ,0' ,WW, ,Wo
W.  b>  .Wo  NWN  bWMWK  0WMWX  ,,,  OWl  0WN
W;  NN  <WN,  .  ,WWMWK  0WMW' ,WWW< .WWl  .. .
MNXXWWXXNMWWN000NWWMMWWXXWWMMXXNWMWWXXWWWWX00XW
MWWWMMWWWMMMWWWWWMMMMMMWWWMMMWWWMMMWWWMMMWWWWWM
"#,
    r#"MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
WWWMMMMMWWWWWWWMMWWWWMWWWWWWWWWMMMMWWWWMMMMMMMWWWWWM
XKNMMMMMNKXNKKWWWWKKNWXKKKKKKKXWMMMNKKXWMMMMWWN0O0XW
, <WWWWW' ;o  KWW0  bW         NWWW.   NWMMWk.     <
l ,Wb>xW. do  KWW0  bWWWW, .WWWWMWb ., <WMWO  'XWWKk
0  N   W  Ko  KWW0  bWWWW, .WWWMWN  ok  XWW'  NW0OOO
W  o l o  Wo  KWW0  bWMMW, .WMMMW>  oo  ;WW'  NW.
W,  .W.  ,Wx  xWWd  kWMMW, .WMMWN  ,,,,  0Wk  ;WWO
Wl  bWl  oWW;  ..  ;WWMMW, .WMMW;  NWWW. ,WWd.  .  .
WNOOWWWOONWWWNkxxkNWWMMMWKO0WMMW0OKWMMMXOOWWWWKxdxKW
MWWWWMWWWWMMMWWWWWWMMMMMMWWWMMMMWWWMMMMWWWMMMWWWWWWW
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
"#,
    r#"MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
WWWMMMMMMWWWWWWWMMMMWWWMWWWWWWWWWWWWMMMWWWWWWMMMMMWWWWWWWWW
;.;WMMMMWd..N,.,WWMW,..NN..........XWMWN,..,WWMMMWWb,.  .,0
>  WWMMWW; .M.  WWMW.  NWlll,  ,lllNWMW>    lWMMWK.  'obl,b
k  KO  <W, 'M.  WWMW.  NWWWWo  >WWWMMWX  <;  NWMW,  oWWWWMW
W  b;   N  bW.  WWMW.  NWMMWo  >WMMMMW;  XK  ;WMW   XWx<<<>
W, ; .l l  KW.  NWMW.  NWMMWo  >WMMMW0   ;;   KWW.  0Wo,
W>   bN    WW,  kWWK   WWMMWo  >WMMMW,  ;;;;  'WWl  ,XWW'
Wk   NW;  ,WW0.   .   xWMMMWo  >WMMWk  ;WWWW,  OWWo   ..  .
WWxxOWWXdxKWMWWOboobkWWMMMMWKddKMMMW0xdXWMMWXdx0WMWWObood0W
MWWWWMMWWWWMMMWWWWMWWWMMMMMMWWWWMMMMWWWWMMMMWWWWMMMWWWMWWWW
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
"#,
    r#"MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
WWWWMMMMMMWWWWMWWWWMMMMWWWWMWWWWWWWWWWWWWMMMMWWWWWWMMMMMMMMWWWWWWWWM
kddWWMMMMWWddkWxddXWMMWxddXW0dddddddddddWWMMWNdddxWWMMMMMWWXxollbOWM
;  kWWWWWWK  ,W   xWMWW   xW;           KWMMW<    bWMMMMWO,       'W
d  lWXOOXWd  lW   xWMWW   xWNXXXo   XXXXWWMWX  .   NWMMWk   ;0NWXd>W
X  ,M;  ,W>  OW   xWMWW   xWWWWWd   WWWWMMMW'  x>  >WMMW.  ,WWWWWWWW
W. .N    K'  WW   xWMWW   xWMMMWd   MMMMMMWO  .WN   XWWW   lWW<;;;;K
W;  b '< ;. ,WW   xWMWW   xWMMMWd   MMMMMMW,  .;;.  ;WWW   ;WW,.   O
Wd    OK    >WW   oWWWX   OWMMMWd   MMMMMWx   ....   KWW<   KWMW'  O
WX   .WW,   kWWl   ,;;   ,WWMMMWd   MMMMWW.  bWWWW;  ,WWW'   ,;;   O
MW,..dWWO...NWMWk'.   .,oWWMMMMWk..,MMMMWk..,WWMMWX...KWWWO;.   .,oW
MMWWWWMMWWWWWMMMWWWWWWWWWWMMMMMMWWWWMMMMMWWWWWMMMMWWWWWMMMWWWWWWWWWW
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
"#,
    r#"MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
WWWWMMMMMMMMWWWWWWWWWWMMMMWWWWWMMWWWWWWWWWWWWWWMMMMMMWWWWWWMMMMMMMMMWWWWWWWWWWM
l''oWMMMMMMWK'''Nx'''OWMMWN'''lWW>''''''''''''<WMMMMWd''''xWMMMMMMWWNd;,...,>OW
>  .WMWWWWMWb   W>   bWMMWX   ,WW.            .WMMMWN      WWMMMMWW<         .W
O   WWNKKXWW<  ,W>   bWMMWX   ,WWNNNNN    NNNNNMMMMW<  ..  lWMMMMW,   ,OWWWXd'W
W   OW'   XW,  >W>   bWMMWX   ,WMWWWWW.   WWWWWMMMWX   do   NWMMWk   .WWWWWWWWW
W,  oN    <W.  OW>   bWMMWX   ,WMMMMMW.   WWMMMMMMW;  .WN   ;WMMW<   >WMN00000X
W>  ;d  ;  W   WW>   bWMMWX   ,WMMMMMW.   WWMMMMMW0   lWW>   XWMW;   oWMl     <
Wk  ., ;K  l  .WWl   bWMMWX   ,WMMMMMW.   WWMMMMMW,          'WMWb   ,WM0oo   <
WW     0W,    ;WWd   ;WWWWd   ;WMMMMMW.   WWMMMMWk   ,''''.   0WWN    bWWWW.  <
MW,   .WWk    xWWW.   .';,    KWMMMMMW.   WWMMMMW.   NWWWWX   ,WMW0.   .';'   <
MW>   dWMW.   XWMWW>.      .;XWMMMMMMW.   WWMMMWx   >WMMMMW;   kWMWWo,      .;0
MMWWWWWMMMWWWWWMMMWWWWXK0KNWWWMMMMMMMMWWWWMMMMMMWWWWWMMMMMMWWWWWMMMWWWWXK0KNWWW
MMWWWWMMMMWWWWMMMMMMMWWWWWWMMMMMMMMMMMWWWWMMMMMMWWWWMMMMMMMWWWWWMMMMMMWWWWWWMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
    "#,
];

#[derive(Debug)]
pub(crate) struct Banner;

impl Banner {
    /// Get the banner size for the size of the terminal
    pub(crate) fn get(mut rect: Rect) -> String {
        rect.height = rect.height.checked_sub(4).unwrap_or(rect.height);
        format!(
            "{}\n{} ({})\nAuthor: {}\nHomepage: {}",
            BANNERS
                .iter()
                .rev()
                .find(|banner| {
                    // super::notify(
                    //     format!(
                    //         "w:{},h:{}-w:{},h:{}",
                    //         rect.width,
                    //         rect.height,
                    //         banner
                    //             .lines()
                    //             .max_by(|x, y| x.len().cmp(&y.len()))
                    //             .unwrap_or_default()
                    //             .len(),
                    //         banner.lines().count()
                    //     ),
                    //     None,
                    // );
                    usize::from(rect.height) > banner.lines().count() - 1
                        && usize::from(rect.width)
                            > banner
                                .lines()
                                .max_by(|x, y| x.len().cmp(&y.len()))
                                .unwrap_or_default()
                                .len()
                })
                .unwrap_or(&BANNERS[0]),
            env!("CARGO_PKG_DESCRIPTION"),
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_AUTHORS"),
            env!("CARGO_PKG_REPOSITORY")
        )
    }
}
