import scrapy
from test_feature.items import FeatureItem

class GetFeaturesSpider(scrapy.Spider):
    name = "getfeatures"
    start_urls = ["https://doc.rust-lang.org/stable/unstable-book"]

    def parse(self, response):
        lang_features = response.xpath("//body[1]/nav[1]/div[1]/ol[1]/li[5]/ol[1]/li/a/text()")
        
        for feat in lang_features:
            item = FeatureItem()
            item['name'] = feat.get().strip()
            yield item
        
        lib_features = response.xpath("//body[1]/nav[1]/div[1]/ol[1]/li[7]/ol[1]/li/a/text()")
        
        for feat in lib_features:
            item = FeatureItem()
            item['name'] = feat.get().strip()
            yield item