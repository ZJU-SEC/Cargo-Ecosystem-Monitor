import scrapy

class FetchCratesSpider(scrapy.Spider):
    name = "fetchcrates"
    start_urls = ["https://crates.io/api/v1/crates"]

    def parse(self, response):
        pass