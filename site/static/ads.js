// AdSense の広告ユニットを描かせる（インラインの push だと CSP で止まるので外部ファイルにしてある）。
(function () {
  var units = document.querySelectorAll('ins.adsbygoogle');
  window.adsbygoogle = window.adsbygoogle || [];
  for (var i = 0; i < units.length; i++) {
    try {
      window.adsbygoogle.push({});
    } catch (e) {
      // 広告ブロッカーなどで読み込めなくても、ページは動かし続ける
    }
  }
})();
